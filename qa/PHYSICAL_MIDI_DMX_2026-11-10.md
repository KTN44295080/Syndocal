# Physical MIDI fader and USB-DMX fixture check — 2026-11-10

Operator present at the rig; Claude drove the native UI over a loopback CDP
port. Isolated QA build `c5/build-b7deae29754c-normal-2611101805`
(profile `jp.seraf.ktn.syndocal.qa.c5xb7deae29n2611101805`, machine role
seeded `Both`). Evidence: `target/qa/physical-midi-dmx-20261110/`.

## Rig

- COM5: FTDI `VID_0403+PID_6001` (Open DMX type), U0.
- Fixture: owner-confirmed 4-channel PAR, mode Dimmer/R/G/B, address 1.
  (The 08-29 record of an F3200A laser at DMX1 no longer applies.)
- MIDI: SMC-Mixer over USB. WinMM inputs: `SMC-Mixer-bt`, `CustomMIDI1`,
  `SMC-Mixer`, `MIDIIN2 (SMC-Mixer)`. In its current (MCU) mode fader 1 sends
  **Pitch Bend channel 1** (`E0 00 xx`), received on both `SMC-Mixer` and
  `MIDIIN2 (SMC-Mixer)`; nothing on `SMC-Mixer-bt`. Owner: motor/LED feedback
  needs the controller's CC mode (not tested here).

## Results

| Step | Result |
| --- | --- |
| Patch `Generic RGB / Dimmer RGB 4ch` at U0 A1, group `Par` | PASS |
| Setup > I/O > DMX: new U0 USB-DMX row on COM5, Apply | **FAIL first time**: `DMX Apply failed at start USB universe 0 … OutputControl rejected (publication_failed); physical output state is unknown.` Bind completed; start failed. After an app restart the persisted row only needed Enabled + Apply: `DMX output settings applied.`, state `Sending`. Not reproduced yet; the native error string is not surfaced (typed rejection only). |
| Control > Lighting: select `Par`, Dimmer Full, colour red | PASS — U0 raw 1=255, 2=255, 3=0, 4=0; **owner saw the fixture lit red**. |
| MIDI Learn on DIMMER with fader 1 | PASS after selecting `SMC-Mixer`: `MIDI mapped PitchBend ch 1 #0 to DIMMER`. Owner moved the fader: **fixture brightness followed**; MIDI Control `Connected`, U0 ch1 254 at the fader top. |

## Findings and fixes

1. **Faders panel not learnable with a group selected** (owner: "フェーダーが
   ピンクにならなくね？"). `FaderGridPanel` returned no Learn target whenever a
   group was selected, and the Dimmer panel's level pad/slider never had one.
   Fixed in `6b14bad0`: a group-selected fader maps `FixtureAttribute` for
   every member (the same fixtures `SetGroupAttribute` writes); otherwise the
   reference fixture.
2. **MIDI input preselected the first port** (`SMC-Mixer-bt`, idle), so Learn
   timed out (10 s) while the owner moved the USB fader. Fixed in `cfdbdf65`:
   the last chosen input name is remembered machine-locally and preselected.
3. Learn timed out after 10 s; a human reading an instruction first easily
   misses it. Recorded, not changed.
4. A new project ships a U0 Art-Net row to `127.0.0.1` that reads `Sending`
   next to the USB row; the owner asked whether two U0 rows were intended.
   They are (one universe to two destinations), but the loopback default is
   confusing. Recorded, not changed.
5. USB-DMX first Apply failure (above): to reproduce on the fix build.

## Fix build `cfdbdf65` (fresh profile `jp.seraf.ktn.syndocal.qa.c5xcfdbdf65n2611102020`)

- Fresh patch + new U0 USB-DMX COM5 row + Apply: `DMX output settings
  applied.`, `Sending` on the first try. The earlier first-Apply failure did
  **not reproduce**; cause still unknown.
- Fix 1: with group `Par` selected, Learn mode now marks the Faders panel CH1
  fader pink (`Dimmer · Par`) — `fix-build/01-learn-pink.png`. With the
  operator back at the controller: `MIDI mapped PitchBend ch 1 #0 to Dimmer ·
  Par`; MIDI Control `Connected`; U0 ch1 followed the fader 0–254 over 15 s
  (`fix-build/fader-dimmer-samples.json`). **PASS.** (An earlier 2 + 10 minute
  wait saw no MIDI because the operator was away; raw WinMM also saw 0.)
- Fix 2: the first launch still preselects `SMC-Mixer-bt` (nothing
  remembered yet); choosing `SMC-Mixer` stored the name, and after an app
  restart the MIDI input was preselected as `SMC-Mixer`. **PASS.**

## Not covered

MIDI feedback (CC mode), DMX timing waveform (no analyser), RDM (no devices).
