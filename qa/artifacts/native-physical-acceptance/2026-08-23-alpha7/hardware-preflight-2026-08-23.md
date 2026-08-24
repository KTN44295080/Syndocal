# Windows hardware preflight — 2026-08-23

Captured at `2026-08-23T07:42:19+09:00` from the current Windows machine by
read-only OS enumeration. This is preparation evidence, not post-D3 release
acceptance and not proof of signal output, timing, disconnect recovery, or soak.

## Native process

- Exact checkout executable:
  `C:\Users\kouty\Documents\kdmx\target\release\syndocal.exe`
- PID `103260`, product version `1.2.0-alpha.7` through its WebView process.
- The preceding UI pass recorded one responsive maximized `1920x1032` Syndocal
  window. No output or device connection was started during this OS inventory.

## Stream Deck Pedal

- The running Elgato Stream Deck host supplies `Stream Deck Pedal` to its plugin
  processes as a three-control device (`type: 5`).
- Exact host-reported device identity:
  `75b8f0879b24a2c75ab36d3b8a2524b1`.
- This proves current host recognition only. Pedal press/release mapping into the
  Syndocal DJ Agent, exact event identity, debounce, reconnect, and the authored
  Release/Loop actions remain unverified.

## MIDI and audio devices observed present

- MIDI candidates exposed in Syndocal and present in Windows include
  `SMC-Mixer-bt`, `CustomMIDI1`, and `Ampero Mini Subdevice`.
- Windows also reports `SMC-PAD` Bluetooth MIDI IN/OUT and Bome/teVirtualMIDI
  endpoints as present.
- Physical/audio candidates present include `E2x2`, `Ampero Mini`, `TOPPING USB
  DAC`, Elgato Virtual Audio endpoints, and their capture/playback endpoints.
- No MIDI/MTC port was opened and no audio stream was started in this preflight.

## ASIO registration

Both 64-bit and WOW6432Node ASIO registries contain the same eight driver names:

- `Ableton Move`
- `Ableton Push`
- `DDJ-FLX10 ASIO Driver`
- `HOTONE AUDIO USB Audio Device`
- `MOTU M Series`
- `Realtek ASIO`
- `TOPPING Pro USB Audio Device`
- `Topping USB Audio Device`

Registry presence does not prove Syndocal enumeration, bridge licensing/package
identity, open/start/callback behavior, sample-rate/buffer/channel negotiation,
exclusive ownership, XRUN/disconnect recovery, or audible routing.

## DMX serial and DJ processes

- The first `Win32_SerialPort` query returned no rows, but that provider was not
  authoritative for this USB virtual COM device. A follow-up PnP and registry
  cross-check found `USB Serial Port (COM3)` present with status `OK`, instance
  `FTDIBUS\VID_0403+PID_6001+A&A5D719&0&8\0000`, and
  `HKLM\HARDWARE\DEVICEMAP\SERIALCOMM` mapping `\Device\VCP0` to `COM3`.
  This proves current Windows enumeration only; Syndocal open/write/cadence,
  blackout/release, and disconnect recovery remain unverified.
- Syndocal UI remained on stopped Art-Net `127.0.0.1:6454`; no packet or DMX frame
  was emitted.
- No running process named rekordbox or a Syndocal/DJ Link Agent was found. The
  complete rekordbox-to-Agent-to-Syndocal and Pedal path remains open.

## Web Remote / DJ Link native stop condition

- The native UI was reverified as the exact checkout executable, one responsive
  maximized `1920x1032` window. Web Remote was stopped at `127.0.0.1:9100` with
  trusted-LAN access disabled. DJ Link was enabled in the UI, but starting it
  failed closed because no explicit non-loopback Show-LAN bind IP was selected.
- Windows reported active Ethernet IPv4 `192.168.1.34`. When the native UI's
  `Trusted LAN access` checkbox was enabled in preparation for selecting that
  exact interface, `syndocal.exe` terminated at `2026-08-23T07:56:16+09:00`.
  Windows Application Error event 1000 recorded exception `0xc000001d`, fault
  offset `0x0000000000ae7d60`, process `0x1935c`; WER report ID
  `93d13146-13b5-489e-a727-96b27e6d9650`.
- Disassembly of the matching `1.2.0-alpha.7` executable/PDB maps the fault RVA
  to an `ud2` refcount-invariant branch in
  `tauri_runtime_wry::Context<EventLoopMessage>::clone`. The checkbox's frontend
  path updates signals and triggers `remote_access_urls`; it does not directly
  start Web Remote. The exact native root cause and current-source reproduction
  are unresolved, so DJ Agent launch was intentionally stopped before MIDI,
  Pedal, WebSocket, or DLL-injection activity.
- A same-day Application log audit found that this was not a one-off checkbox
  crash. The same exact `1.2.0-alpha.7` executable timestamp failed with
  `0xc000001d` at the same RVA `0x0000000000ae7d60` at `07:01:34`, `07:21:10`,
  `07:34:52`, and `07:56:16`. An earlier alpha.7 build also failed with
  `0xc000001d` at a different build-relative RVA at `00:30:27`. Trusted-LAN is
  therefore a reproducible trigger, not proof that the defect is confined to
  that checkbox. The rebuilt native gate must stress representative async Tauri
  queries and idle/close behavior in addition to the LAN toggle.
- A later read-only network preflight found no process listening on TCP `9100`
  and no existing Windows Firewall rule whose display name matched Syndocal or
  KDMX. The active Show-LAN candidate remains Ethernet interface index `28`,
  IPv4 `192.168.1.34`, gateway `192.168.1.1`; the WSL and link-local addresses
  are not accepted as Show-LAN bind targets. Any firewall consent or rule
  created by the rebuilt native server must be recorded as a separate system
  mutation during acceptance.

## Remaining acceptance

First close and reverify the Web Remote trusted-LAN crash on the integrated D3
native build. Then repeat with the exact rebuilt executable:
real editor plus LED/projector output, DMX channel/cadence/blackout/release,
MIDI/MTC, DJ Agent plus rekordbox and Pedal actions, WASAPI and separately licensed
ASIO audio, deliberate disconnect/reconnect, and the one-hour integrated soak.
