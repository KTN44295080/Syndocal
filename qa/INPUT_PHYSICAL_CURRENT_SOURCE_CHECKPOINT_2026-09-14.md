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

## Takeover continuation — current-source input recheck — 2026-09-14

At HEAD `69c86ac4`, the DVC MIDI shortcut check passed `39` assertions, the
DVC DMX shortcut check passed `41`, and frontend command routing passed with
`133` renderer, `31` server-authoritative, `28` raw, and `479` facade
dispatches. The prior SMC-Mixer release test remains the only recorded physical
slice; it is not repeated here. No OSC/TouchOSC/Remote client, Clock/MTC,
latency, reconnect, or new MIDI traffic was used. `INPUT-PHYSICAL-001` remains
`Open`.

## Takeover continuation — current-source input-contract recheck after Video repair — 2026-09-14

At current source HEAD `4dc18a7e`, the input route contracts were rerun:

```text
pnpm.cmd run check:dvc-midi-shortcuts
dvc midi shortcuts ok: 39 assertions

node scripts/check-dvc-dmx-shortcuts.mjs
dvc dmx shortcuts ok: 41 assertions

pnpm.cmd run check:frontend-command-routing
frontend command routing exact: 133 renderer mutations, 31 server-authoritative
mutations, 28 raw dispatches, 479 facade dispatches
```

All commands exited `0`. Generation/authority routing and the narrow input
reader boundary remain enforced. No new MIDI traffic, OSC/TouchOSC or Web
Remote client, Clock/MTC path, latency trial, reconnect/replacement run, or
native UI/device workflow was performed. The previously recorded SMC-Mixer
enumerate/open plus safe All Notes Off slice remains the only physical slice;
`INPUT-PHYSICAL-001` remains `Open` pending the named physical input matrix,
reconnect timing, raw logs, and operator observations.

## Current-host physical MIDI recheck — 2026-09-15

The safe SMC-Mixer recheck was attempted again with the exact MSVC
`14.44.35207` x64 linker and the explicit selectors
`SYNDOCAL_TEST_MIDI_INPUT=SMC-Mixer` and
`SYNDOCAL_TEST_MIDI_OUTPUT=SMC-Mixer`:

```text
Finished `release` profile [optimized] target(s) in 0.85s
MIDI inputs: [CustomMIDI1]
MIDI outputs: [Microsoft GS Wavetable Synth, CustomMIDI1]
no MIDI input matched 'SMC-Mixer'
test result: FAILED; 0 passed; 1 failed; 0 ignored
```

The test stopped before opening an output or sending the safe All Notes Off
message, so this attempt produced no MIDI traffic. This is an environmental
availability result, not a product regression: the previously recorded
SMC-Mixer enumerate/open and `B0 7B 00` slice remains the last successful
physical observation. `INPUT-PHYSICAL-001` remains `Open` pending the named
MIDI/OSC/Remote client matrix, device identity, reconnect/latency/feedback and
Clock/MTC evidence. Before retrying, restore or reconnect the named SMC-Mixer
input/output and capture the complete port inventory.

## Continuation — current-source input-routing recheck — 2026-09-15

At current source HEAD `33ad01d4`, with the exact MSVC `14.44.35207` x64 linker
confirmed first by `where.exe link.exe`, the input-related source contracts
all exited `0`:

```text
pnpm.cmd --dir app run check:dvc-midi-shortcuts
39 assertions
node app/scripts/check-dvc-dmx-shortcuts.mjs
41 assertions
pnpm.cmd --dir app run check:frontend-command-routing
133 renderer, 31 server-authoritative, 28 raw, 479 facade dispatches
```

No MIDI/OSC/TouchOSC/Remote client, Clock/MTC path, latency trial, reconnect
run, or native device workflow was performed. The last successful physical
SMC-Mixer slice remains the prior safe `B0 7B 00` observation;
`INPUT-PHYSICAL-001` remains **Open**.

## Continuation — PnP-present but MIDI-port-absent recheck — 2026-09-15

The host PnP inventory currently reports `SMC-Mixer (Bluetooth MIDI IN/OUT)`
and the paired Bluetooth device as present. That presence did not make the
endpoints available to the production `midir` layer. With the exact pinned
MSVC `14.44.35207` linker and the explicit `SMC-Mixer` selectors, the ignored
release test again enumerated only `CustomMIDI1` for input and
`Microsoft GS Wavetable Synth` plus `CustomMIDI1` for output, then failed before
opening a port or sending bytes. The test exited with `0 passed / 1 failed / 0
ignored`.

This is a host MIDI-service/Bluetooth availability discrepancy, not evidence
that the PnP entry is an opened production endpoint. No MIDI traffic was sent
in this attempt. `INPUT-PHYSICAL-001` remains **Open**; resume by restoring
the SMC-Mixer endpoint in the `midir` inventory, then rerun the named matrix
with raw input/output logs and operator observations.

## Continuation — reconnected SMC-Mixer production MIDI slice — 2026-09-15

After the SMC-Mixer was reconnected, the current Windows PnP inventory showed
both the named SMC-Mixer MIDI endpoints and the paired Bluetooth MIDI services:

```text
SMC-Mixer (Bluetooth MIDI OUT)
SMC-Mixer
MIDIIN2 (SMC-Mixer)
MIDIOUT2 (SMC-Mixer)
SMC-Mixer (Bluetooth MIDI IN)
SMC-Mixer (USB PnP; USB\\VID_4353&PID_4B4D...)
```

The production `midir` inventory then exposed the selected endpoints:

```text
MIDI inputs: [CustomMIDI1, SMC-Mixer, MIDIIN2 (SMC-Mixer)]
MIDI outputs: [Microsoft GS Wavetable Synth, CustomMIDI1, SMC-Mixer, MIDIOUT2 (SMC-Mixer)]
```

With the exact pinned MSVC `14.44.35207` x64 linker (the pinned linker was
first in `where.exe link.exe`) and
`SYNDOCAL_TEST_MIDI_INPUT=SMC-Mixer` /
`SYNDOCAL_TEST_MIDI_OUTPUT=SMC-Mixer`, the production ignored release test
was rerun:

```text
cargo test -p io --release --locked -j 1 physical_midi_ports_enumerate_open_and_send_feedback -- --ignored --nocapture --test-threads=1
test result: ok. 1 passed; 0 failed; 0 ignored; 186 filtered out
```

The test enumerated and opened the selected input/output and sent exactly one
safe channel-1 `B0 7B 00` All Notes Off message. The command exited `0`; no
other MIDI traffic was intentionally generated. This closes the previously
unavailable current-host enumerate/open/safe-feedback slice and supersedes
the immediately preceding PnP-present-but-`midir`-absent attempt as the latest
availability result.

This does not close `INPUT-PHYSICAL-001`. It does not prove controller
movement, input capture, LED observation, MIDI Clock/MTC, input-to-pixel or
round-trip latency, reconnect/replacement, OSC/TouchOSC, Web Remote,
native UI routing, DMX, or venue acceptance. The next action remains the
named physical input matrix with raw logs, timing, reconnect evidence, and
operator observations.

## Continuation — host-level physical MIDI input observation — 2026-09-15

To check whether the reconnected unit was producing input data, a temporary
Windows WinMM observer opened both currently exposed SMC-Mixer input aliases:
`SMC-Mixer` (index 1) and `MIDIIN2 (SMC-Mixer)` (index 2). It performed no
output send and changed no device or product configuration. During the
bounded 15-second observation it received `308` `MIM_DATA` messages, `154`
through each opened alias. Representative raw channel/controller/value bytes
were `B0 15 41`, `B0 16 41`, and `B0 17 41`; the process exited `0` after
stopping, resetting, and closing both inputs.

Both aliases delivered the same observed data, so this is one connected-unit
host observation with duplicate Windows endpoint presentation, not proof of
two independent physical inputs. It proves that MIDI CC-shaped data reached
the Windows input layer after reconnection. It does not prove that Syndocal's
production listener selected the intended alias, that a mapping or visual
Learn action consumed it, or that controller movement, Clock/MTC, LED
feedback, latency, reconnect/replacement, OSC/TouchOSC, Web Remote, native
UI routing, DMX, or venue acceptance passed. `INPUT-PHYSICAL-001` remains
**Open**.

## Latest production-port availability recheck — 2026-09-15

After the preceding successful reconnect slice, the host still reported the
paired SMC-Mixer and both named Bluetooth MIDI PnP endpoints as `Status=OK`,
and the Windows MIDI service was `Running`. The current production `midir`
enumeration nevertheless returned only:

```text
MIDI inputs: [CustomMIDI1]
MIDI outputs: [Microsoft GS Wavetable Synth, CustomMIDI1]
```

With the exact pinned MSVC `14.44.35207` linker and explicit
`SYNDOCAL_TEST_MIDI_INPUT=SMC-Mixer` / `SYNDOCAL_TEST_MIDI_OUTPUT=SMC-Mixer`,
`physical_midi_ports_enumerate_open_and_send_feedback` exited `101` with
`no MIDI input matched 'SMC-Mixer'`. The test failed before opening a port or
sending bytes. This is a reproducible host MIDI-service/Bluetooth endpoint
availability discrepancy, not a reason to treat the PnP entry as an opened
production endpoint. The earlier successful reconnected run and its safe
`B0 7B 00` message remain retained evidence; this latest failure is recorded
as the current availability result. `INPUT-PHYSICAL-001` remains **Open**
pending stable `midir` availability and the named movement, feedback, clock,
reconnect, latency, and native-routing matrix.

## Windows MIDI service restart boundary — 2026-09-15

The paired SMC-Mixer and its three current PnP entries were still present with
`Status=OK`, and `midisrv` was `Running`. A restart attempt was made before a
final production `midir` retry, but Windows rejected the service control
operation with:

```text
Service 'Windows MIDI サービス (midisrv)' cannot be stopped ... Cannot open
'midisrv' service on computer '.'.
```

No service state was changed by this attempt. The subsequent `midir` retry
still enumerated only `CustomMIDI1` and failed before opening or sending any
SMC-Mixer endpoint. This is retained as a host recovery boundary, not as a
MIDI acceptance result; `INPUT-PHYSICAL-001` remains **Open**.
