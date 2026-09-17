# AI3 native ingress current-source checkpoint — 2026-09-14

- Marker: `AI3-NATIVE-INGRESS-001`
- Branch: `codex/showclock-review-20260912`
- Base: `6f8c5a7dd63eb5a70cd42ac6b9e1ec15982f6452`
- Product code change: none in this checkpoint

## Current-source verification

The canonical renderer/native admission and output safety seams were checked
against the current source:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 479 facade dispatches |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 539 commands; 18 negative fixtures rejected; source fingerprint `a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab` |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS — output-control and Standby Sync contracts |
| `pnpm.cmd --dir app run check:output-ownership` | PASS |
| `pnpm.cmd --dir app run check:safety-blackout-runtime` | PASS |

These checks show that current source routes mutations through the inventoried
admission/ownership/safety seams and rejects the checked negative fixtures.

## Current-host physical MIDI slice — 2026-09-14

The current-host SMC-Mixer slice recorded in
`qa/INPUT_PHYSICAL_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md` opened the
production `midir` input `SMC-Mixer` (index 1) and output `SMC-Mixer` (index 2)
and sent one safe channel-1 All Notes Off message (`B0 7B 00`). The ignored
release test passed `1/1` with zero failures or ignored tests. This is partial
native MIDI transport/feedback evidence for AI3; it is not controller
movement, Clock/MTC, latency, reconnect, OSC/Remote, DMX, Art-Net, or
end-to-end native ingress acceptance.

## Acceptance boundary

`AI3-NATIVE-INGRESS-001` remains `Open`. Apart from the bounded SMC-Mixer MIDI
slice above, no native OSC/DMX/Remote client, real Art-Net node, fixture, or
other physical ingress was connected or driven in this checkpoint. Static
inventory and one safe MIDI feedback message do not prove controller movement,
Clock/MTC, reconnect, latency, complete feedback, or venue behavior.

## Resume procedure

Use the actual native clients and devices against the exact current artifact.
Record device identity/topology, accepted and rejected ingress, reconnect and
replacement behavior, output state, and any first failure before reconsidering
the Flow marker.

## Takeover continuation — current-source ingress recheck — 2026-09-14

The five current-source checks were rerun against HEAD `3d50fd7e` after the
takeover:

```text
check:frontend-command-routing: 133 renderer, 31 server-authoritative,
28 raw, 479 facade dispatches — PASS
check-tauri-admission-inventory: 539 commands, 18 negative fixtures rejected,
SHA-256 a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab — PASS
check:output-control-runtime: output-control and Standby Sync contracts — PASS
check:output-ownership: static contract — PASS
check:safety-blackout-runtime: runtime contract — PASS
```

The rerun opened no native client, device, endpoint, or physical output. It
confirms the current routing/admission/ownership/safety source boundary only;
the bounded SMC-Mixer All Notes Off slice above remains the only current-host
native transport observation.

`AI3-NATIVE-INGRESS-001` remains `Open` for native OSC/DMX/Remote clients,
real Art-Net and fixture behavior, controller movement, Clock/MTC, feedback,
reconnect, latency, and venue evidence.

## Takeover continuation — current-source ingress/admission recheck — 2026-09-14

At current source HEAD `1fae0a52`, the ingress source contracts were rerun:

```text
check:frontend-command-routing: PASS (133 renderer, 31 server-authoritative,
28 raw, 479 facade dispatches)
check-tauri-admission-inventory: PASS (539 commands; 18 negative fixtures rejected)
check:output-control-runtime: PASS (including Standby Sync output-lease UI)
check:output-ownership: PASS
check:safety-blackout-runtime: PASS
check:dvc-midi-shortcuts: PASS (39 assertions)
check-dvc-dmx-shortcuts: PASS (41 assertions)
check:agent-bridge: PASS (11 groups)
```

All eight commands exited `0`. They confirm current-source routing,
admission, output ownership, safety, DVC shortcut, and Agent Bridge boundaries
only. No native OSC/Remote client, real Art-Net node/fixture, Clock/MTC
controller, reconnect path, or new physical output was opened; the existing
SMC-Mixer All Notes Off slice remains partial evidence.

`AI3-NATIVE-INGRESS-001` remains `Open` pending the named native client and
hardware matrix with movement, feedback/Clock/MTC, reconnect, latency, and
venue evidence.

## Takeover continuation — current-source ingress/admission recheck after Video repair — 2026-09-14

At current source HEAD `4b09b1bd`, the ingress, admission, ownership, safety,
shortcut, and Agent Bridge checks were rerun. All eight commands exited `0`:

```text
pnpm.cmd run check:frontend-command-routing
133 renderer, 31 server-authoritative, 28 raw, 479 facade dispatches — PASS

node scripts/check-tauri-admission-inventory.mjs
539 commands; SHA-256 a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab;
18 negative fixtures rejected — PASS

pnpm.cmd run check:output-control-runtime
output-control and Standby Sync contracts — PASS

pnpm.cmd run check:output-ownership
static contract — PASS

pnpm.cmd run check:safety-blackout-runtime
runtime contract — PASS

pnpm.cmd run check:dvc-midi-shortcuts
39 assertions — PASS

node scripts/check-dvc-dmx-shortcuts.mjs
41 assertions — PASS

pnpm.cmd run check:agent-bridge
11 groups; no native or device calls — PASS
```

These results reconfirm the current-source routing/admission/ownership/safety,
DVC shortcut, and Agent Bridge boundaries only. No native OSC/Remote client,
real Art-Net node/fixture, Clock/MTC controller, reconnect path, or new
physical output was opened; the existing SMC-Mixer All Notes Off slice remains
partial evidence. `AI3-NATIVE-INGRESS-001` remains `Open` pending the named
native client and hardware matrix with controller movement, feedback/Clock/MTC,
reconnect, latency, and venue evidence.

## Takeover continuation — bounded Agent Bridge ingress corpus — 2026-09-14

At current source HEAD `62efdcb9`, the Agent Bridge wire admission test was
extended with a deterministic 512-case hostile request corpus. The corpus
contains one canonical `fixtures.list` request, one request over the 64 KiB
wire limit, 128 non-UTF-8 random frames, 128 truncations of the canonical
request, and 254 strict-shape variants with an unknown top-level field. Each
case is bounded at `MAX_REQUEST_BYTES + 1` and is evaluated through JSON decode
plus `Request::command()` inside `catch_unwind`.

The exact Windows release test used the repository-required MSVC procedure:

```text
vcvars64.bat -vcvars_ver=14.44
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe
where.exe link.exe -> the same 14.44.35207 linker
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 agent_bridge_ -- --nocapture --test-threads=1
```

```text
Finished `release` profile [optimized]
agent bridge hostile request corpus: 512 cases, 511 rejected, 0 panics, max_bytes=65537
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 1918 filtered out
```

The focused source checks also passed with exit code `0`: frontend command
routing (`133` renderer, `31` server-authoritative, `28` raw, `479` facade),
native admission inventory (`539` commands and `18` negative fixtures rejected),
output-control/Standby Sync, output ownership, safety blackout, DVC MIDI
shortcuts (`39` assertions), DVC-DMX shortcuts (`41` assertions), and Agent
Bridge (`11` groups; no native/device calls). The Windows-native warning ratchet
also passed: baseline and current warnings were both `0` total, including
`0` first-party warnings.

This is current-source parser-boundary evidence only. It does not prove native
OSC/DMX/Remote client admission, a real Art-Net node or fixture, controller
movement, Clock/MTC, complete feedback, reconnect/replacement, latency, or
venue behavior. The existing SMC-Mixer All Notes Off observation remains the
only partial native transport slice. `AI3-NATIVE-INGRESS-001` remains `Open`.

## Resume procedure after this checkpoint

Use the exact current release artifact with the named native clients and
hardware matrix. First capture device identity/topology, then exercise accepted
and rejected ingress, controller movement, feedback/Clock/MTC, reconnect and
replacement, output state, and latency. Record each result and first failure;
only then reassess the Flow marker.

## Continuation — current-source ingress/admission recheck — 2026-09-15

At current source HEAD `17c0ca40`, the Windows-native source gate was rerun
after initializing MSVC 14.44.35207 Build Tools. The pinned linker was printed
and `where.exe link.exe` resolved to the same
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe` first. All ten checks exited `0`:

```text
check:frontend-command-routing: PASS (133 renderer, 31 server-authoritative, 28 raw, 479 facade dispatches)
check-tauri-admission-inventory: PASS (539 commands; SHA-256 a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab; 18 negative fixtures rejected)
check:output-control-runtime: PASS (v12 output commands, lease-bound masters, strict receipts, no-send reconciliation, and Standby Sync output-lease UI)
check:output-ownership: PASS
check:safety-blackout-runtime: PASS
check:dvc-midi-shortcuts: PASS (39 assertions)
check-dvc-dmx-shortcuts: PASS (41 assertions)
check:agent-bridge: PASS (11 groups; no native/device calls)
check:warnings -- --configuration windows-native-release: PASS (baseline/current total 0; first-party 0)
cargo test ... agent_bridge_: PASS (11 passed; 0 failed; hostile corpus 512 cases, 511 rejected, 0 panics, max_bytes=65537)
```

This is current-source routing, admission, ownership, safety, shortcut, Agent
Bridge, warning, and release-test evidence only. No native OSC/Remote client,
real Art-Net node or fixture, Clock/MTC controller, reconnect path, or physical
output was opened. The existing SMC-Mixer observation remains a partial MIDI
transport slice, and the current host still does not provide the complete
native ingress matrix. `AI3-NATIVE-INGRESS-001` therefore remains `Open`.

The Q4 ledger records this recheck as
`EV-AI3-NATIVE-INGRESS-CURRENT-SW-2026-09-15`. The next action remains the
exact release artifact against named native clients and hardware, with device
identity, accepted/rejected ingress, movement, feedback/Clock/MTC, reconnect,
replacement, output state, latency, and first failure retained before any
marker change.

## Physical SMC-Mixer availability correction — 2026-09-15

The current host PnP inventory and `midisrv` service remained healthy, but a
fresh production `midir` enumerate/open test returned only `CustomMIDI1` input
and `Microsoft GS Wavetable Synth` plus `CustomMIDI1` output. The explicit
`SMC-Mixer` selectors therefore failed with `no MIDI input matched
'SMC-Mixer'` before any MIDI bytes were sent. This supersedes neither the
retained successful reconnect/safe-feedback slice nor the direct WinMM input
observation; it records that availability is not stable across attempts.

No controller, Clock/MTC, feedback, OSC/Remote, DMX, or output action was
performed in this failed attempt. `AI3-NATIVE-INGRESS-001` remains `Open`;
resume by restoring stable production-port enumeration and then running the
named client, movement, feedback/clock, reconnect, replacement, and latency
matrix.

## Physical MIDI safe recheck and operator-ingress capture boundary — 2026-09-15

This continuation started from source HEAD `53203be2` after the UI clipping
checkpoint. The current host was rechecked before the test: PnP exposed
`SMC-Mixer` and `MIDIIN2 (SMC-Mixer)` input aliases, `SMC-Mixer` and
`MIDIOUT2 (SMC-Mixer)` output aliases, `CustomMIDI1`, an FTDI USB serial
device on COM5, and `teVirtualMIDI`; the Windows `midisrv` service was
running. This inventory is availability evidence, not a claim that every
endpoint was consumed by Syndocal.

The exact Windows release command used the repository-required MSVC
14.44.35207 x64 linker, verified first with `where.exe link.exe`:

```text
SYNDOCAL_TEST_MIDI_INPUT=SMC-Mixer
SYNDOCAL_TEST_MIDI_OUTPUT=SMC-Mixer
cargo test -p io --release --locked -j 1 physical_midi_ports_enumerate_open_and_send_feedback -- --ignored --nocapture --test-threads=1
```

Result: `1 passed; 0 failed`. The production `midir` path enumerated the
three visible input ports and four visible output ports, opened the named
SMC-Mixer endpoints, and sent only the safe channel-1 All Notes Off message
(`B0 7B 00`). No lighting, blackout, Take, Arm, Take Over, recording, or
other disruptive action was sent.

To make the missing operator step reproducible, this checkpoint adds the
ignored `physical_midi_input_captures_operator_ingress` test in
`crates/io/src/midi.rs`. It opens the named production `midir` input, records
raw timestamps and bytes for a bounded 1–120 second window, and fails closed
when no message arrives. A 30-second run on the current host returned:

```text
capturing physical MIDI input 'SMC-Mixer' for 30s; move one knob or press one button
physical MIDI input 'SMC-Mixer' produced no operator ingress during 30s
test result: 0 passed; 1 failed
```

This is an observed no-movement result, not a product-failure diagnosis and
not accepted input proof. `AI3-NATIVE-INGRESS-001` remains `Open`. The
remaining boundary is a supervised run with actual controller movement,
complete feedback/Clock/MTC, reconnect and replacement, latency, named
OSC/Remote clients, real Art-Net/DMX hardware, and venue evidence. The test
must be rerun while an operator moves a control; the raw message and
timestamp output must be retained before any ingress claim is advanced.

## Physical SMC-Mixer bridge installation and WinMM recheck — 2026-09-18

The 2026-09-18 production capture attempt did not reach its 30-second
operator window. The first invocation was corrected to quote the `cmd.exe`
environment assignments; the second invocation still failed before opening a
port. A separate production-port enumeration printed only:

```text
MIDI inputs: [CustomMIDI1]
MIDI outputs: [Microsoft GS Wavetable Synth, CustomMIDI1]
no MIDI input matched 'SMC-Mixer'
```

Read-only Windows PnP inventory simultaneously reported `SMC-Mixer
(Bluetooth MIDI IN)`, `SMC-Mixer (Bluetooth MIDI OUT)`, and the paired
`SMC-Mixer` Bluetooth device as `OK`. This proves an OS/PnP endpoint exists;
it does not prove that the product's `midir`/WinMM backend can open it.

The official M-VAVE Windows `Sinco Connector` installer was downloaded to a
temporary directory, Defender-scanned with no detection, and installed with
reboot suppressed. Its built-in x86 diagnostic then reported that the required
Bome BMIDI 2 bus DLL/driver was missing. The official Bome
`BMIDI_Driver_2.1.0.44.exe` was downloaded, its published MD5
`f9b4f6f3894dceddb54398a055948bc8` matched, its Authenticode signature was
`Valid`, and it was installed successfully. A second connector diagnostic
reported:

```text
[OK] Native x86 load: C:\Windows\SysWOW64\bmidilib2.dll
[OK] Bome BMIDI 2 bus driver is installed.
[OK] Native x86 load: C:\Program Files (x86)\Bt Midi Connector\bmidistatic2.dll
```

The `BTMidiConnector.exe` process is running and responsive, but its device
connection action has not yet been invoked; the subsequent Syndocal WinMM
enumeration remains `CustomMIDI1` only. No MIDI bytes, lighting command,
blackout, Take, Arm, Take Over, recording, or other disruptive output was
sent in this recheck. The available computer-use surface did not expose the
Connector window for a safe observed click, so the next action is an operator
click on the Connector's `Connect Device` action, followed by a fresh
`midir` enumeration and a supervised 30-second capture with one knob/button
movement.

`AI3-NATIVE-INGRESS-001` remains `Open`. This checkpoint records the
driver/bridge preflight and its current WinMM boundary only; it does not
establish Bluetooth forwarding, controller movement, feedback, Clock/MTC,
reconnect, latency, OSC/Remote, DMX/Art-Net, or venue acceptance.

## Current production MIDI recheck and supervised-capture boundary — 2026-09-18

At current source HEAD b89737b2, the production midir path was rerun after
the Connector preflight. The exact Build Tools MSVC 14.44.35207 x64 linker
was pinned and resolved first by where.exe link.exe. Port enumeration and
the safe feedback/open test passed:

```text
MIDI inputs:
CustomMIDI1, SMC-Mixer, MIDIIN2 (SMC-Mixer), SMC-Mixer-bt
MIDI outputs:
Microsoft GS Wavetable Synth, CustomMIDI1, SMC-Mixer,
MIDIOUT2 (SMC-Mixer), SMC-Mixer-bt
test result: 1 passed; 0 failed; 0 ignored
```

The named SMC-Mixer input was then opened through the production midir
capture test for a bounded 30-second window. The test was intentionally
fail-closed and produced no message:

```text
capturing physical MIDI input 'SMC-Mixer' for 30s; move one knob or press one button
physical MIDI input 'SMC-Mixer' produced no operator ingress during 30s
test result: 0 passed; 1 failed; 0 ignored
```

No MIDI output, lighting, blackout, Take, Arm, Take Over, recording, or other
disruptive action was performed by the capture run. This recheck proves that
the connector-visible endpoints are currently enumerable/openable, but it
does not prove Bluetooth forwarding or operator ingress. `AI3-NATIVE-INGRESS-001`
remains `Open`; the next required step is a supervised capture while an
operator moves a physical knob/button, followed by complete feedback,
Clock/MTC, reconnect/replacement, latency, OSC/Remote, and DMX/Art-Net
evidence.

## Candidate input scan after bridge connection — 2026-09-18

After the first `SMC-Mixer` capture produced no message, the three current
production input names exposed by WinMM were checked individually:
`SMC-Mixer`, `SMC-Mixer-bt`, and `MIDIIN2 (SMC-Mixer)`. Each named input was
opened through the same production midir capture test for a bounded 30-second
window and each run fail-closed with no operator ingress. No MIDI output or
other disruptive action was performed. This narrows the current observation
to endpoint enumeration/openability; it does not establish current-source
acceptance, Bluetooth forwarding, controller movement, or the remaining
feedback/Clock/MTC, reconnect, latency, OSC/Remote, DMX/Art-Net, and venue
gates. `AI3-NATIVE-INGRESS-001` remains `Open`.

## Connected-bridge capture recheck — 2026-09-18

The BTMidiConnector window was inspected through Windows UI Automation. Its
connected-device list contained `SMC-Mixer`, the connection state exposed the
`Disconnect Device` tooltip, and the status text was `SMC-Mixer Connected.`.
The Bluetooth-device dialog was opened only to inspect its controls and was
closed through its named `ExitButton`; no new pairing or disconnect operation
was invoked.

With that connected state retained, the current production `midir` path was
rerun using the exact MSVC `14.44.35207` x64 linker:

```text
SYNDOCAL_TEST_MIDI_INPUT=SMC-Mixer
SYNDOCAL_TEST_MIDI_CAPTURE_SECONDS=30
cargo test -p io --release --locked -j 1 physical_midi_input_captures_operator_ingress -- --ignored --nocapture --test-threads=1
```

The endpoint opened, but the fail-closed capture timed out with no operator
message:

```text
capturing physical MIDI input 'SMC-Mixer' for 30s; move one knob or press one button
physical MIDI input 'SMC-Mixer' produced no operator ingress during 30s
test result: 0 passed; 1 failed; 0 ignored
exit_code=101
```

This narrows the current boundary to a connected/observable bridge with no
captured physical control movement in the supervised window. It does not
prove Bluetooth forwarding, controller ingress, feedback/Clock/MTC,
reconnect/replacement, latency, OSC/Remote, DMX/Art-Net, or venue behavior.
`AI3-NATIVE-INGRESS-001` remains `Open`; resume with an actual knob/button
movement while this exact capture is running.

## Operator-directed extended capture window — 2026-09-18

With the same UIA-observed `SMC-Mixer Connected.` bridge state retained, a
second operator-directed production `midir` capture was run for 60 seconds.
The prompt explicitly requested one knob or button action after the capture
window opened. No raw message was observed:

```text
SYNDOCAL_TEST_MIDI_INPUT=SMC-Mixer
SYNDOCAL_TEST_MIDI_CAPTURE_SECONDS=60
cargo test -p io --release --locked -j 1 physical_midi_input_captures_operator_ingress -- --ignored --nocapture --test-threads=1
physical MIDI input 'SMC-Mixer' produced no operator ingress during 60s
test result: 0 passed; 1 failed; 0 ignored
exit_code=101
```

This is a longer fail-closed observation, not physical-ingress acceptance.
`AI3-NATIVE-INGRESS-001` remains `Open` pending a captured controller event
and the remaining feedback/Clock/MTC, reconnect/replacement, latency,
OSC/Remote, DMX/Art-Net, and venue evidence.
