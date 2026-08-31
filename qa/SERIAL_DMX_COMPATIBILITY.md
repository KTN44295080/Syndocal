# Serial DMX Compatibility

Updated: 2026-08-31

## Current status

Syndocal can transmit DMX through operating-system serial ports. For the DSF
show, this is the physical lighting primary: Setup > DMX must enumerate and
explicitly confirm the exact machine-local USB-DMX device/protocol. COM/PnP
identity is not authored in `.sdc`; missing, stale, ambiguous, or changed
identity fails closed. Setup exposes port scan/manual COM or `/dev/ttyUSB*`
selection, baud rate, route apply, and route test for:

- ENTTEC DMX USB Pro packet framing (`0x7e`, label 6, 513-byte DMX payload, `0xe7`)
- DMXKing ultraDMX through the compatible USB Pro packet path
- ENTTEC Open DMX and compatible VCP/FTDI serial ports at 250000 baud, 8N2, with a dedicated non-blocking worker, 176 us break and 16 us mark-after-break

Scan Serial now preserves USB VID/PID, product, manufacturer and serial number. Known single-port ENTTEC USB Pro, DMXKing ultraDMX and Open DMX product identities expose a confirmation button for the recommended protocol. Generic FTDI devices and unsupported dual-port Pro Mk2/Ultra Pro products stay on manual selection instead of receiving a potentially unsafe guess.

The engine applies the same per-route reconnect/backoff telemetry used by
network output. An independently configured generic serial route may fail
without blocking an unrelated Art-Net/sACN route. The selected USB-DMX route
in the strict DSF show is coupled to its Art-Net Unity mirror: a USB identity,
write, or flush fault must engage global S0 and publish an Art-Net zero/blackout;
live Art-Net must not continue from a stale frame. Each completed internal
Universe 0 frame is offered to both routes. The Art-Net mirror is ArtDmx at
`127.0.0.1:6454`, wire Universe `0`, exactly `512` channels, at approximately
`40–44 fps`. The exact FT232R/COM3 Open-DMX worker instead keeps the latest
completed U0 frame at its empirically stable `22,764us + 8ms` period (about
`32.5 fps`); it does not claim a distinct physical USB delivery on every 44Hz
engine tick. This is an exact-rig default, not a DMX512/Open-DMX universal
maximum: any increase requires fresh waveform and fixture evidence. USB-DMX
and Art-Net both force channel 500 / `payload[499]` to zero. The two local
Spout senders remain the video path.

The physical route is not accepted merely because a frame entered a worker
queue or because the worker completed its write sequence. Current evidence is
separate: initial S0 all-zero queue acceptance; actual Open-DMX worker BREAK,
MAB, `write_all`, and `flush` completion; live completed-U0 mirror; independent
USB electrical-wire and Art-Net captures; and a separately reviewed,
fixture-specific physical nonzero frame followed by blackout. The currently
attached F3200A laser is all-zero-only until its beam-path safety and exact
34-channel test are explicitly approved; a Mega PAR red frame is not a safe
substitute. Backend output-lease keepalive beyond the 60-second lease TTL and
exact release-artifact/device identity evidence are also pending.

## QLC+ comparison

Official QLC+ 5 documentation describes a broader DMX USB plugin with automatic device detection and manual force modes for Open TX/RX, Pro RX/TX, Pro Mk2, Ultra Pro, DMX4ALL and Vince Tx. It also exposes Pro Mk2 dual DMX outputs and input/MIDI lines. Source: <https://docs.qlcplus.org/v5/plugins/dmx-usb>.

Syndocal currently covers the common output-only Open TX and Pro-compatible TX class through serial/VCP ports. It does **not** yet match QLC+ in:

- FTDI D2XX/native USB discovery independent of virtual COM drivers
- a maintained VID/PID catalog and driver guidance beyond the current conservative product-name recommendation
- ENTTEC Pro Mk2 and DMXKing Ultra Pro dual-port routing
- DMX4ALL and Vince Tx packet protocols
- generic Open RX/Pro RX serial DMX input
- device-specific tuning and physical compatibility coverage

QLC+'s plugin architecture should not be confused with arbitrary application serial messages: every DMX adapter family still needs correct electrical timing or packet framing. Syndocal will not expose an unsafe “send 512 raw bytes at any baud rate” mode as if it were universally compatible.

## Acceptance plan

1. Keep the existing three output modes as the stable serial baseline and use
   the explicitly selected machine-local USB-DMX route as the show's physical
   lighting output.
2. **Complete:** add USB VID/PID/product metadata and a non-destructive protocol recommendation to Scan Serial; manual override remains available. Product-matching tests and the five-reference-browser-viewport contract cover the behavior, with 1920x1080 primary, a 1920x1032 measured-work-area fixture, a 2048x1152 ceiling and 1366x768/1280x720 fallbacks.
3. Add Pro Mk2/Ultra Pro dual-output profiles using verified vendor protocol documentation and per-port routing.
4. Add supported serial DMX input modes to the existing HTP/LTP input merge path.
5. Build packet-codec tests for every profile before allowing a physical port to open.
6. Capture 512-channel waveform and one-hour output evidence for each supported device. Open DMX requires logic-analyzer break/MAB/frame-period verification; Pro-class devices require receiver capture and unplug/replug recovery.

Until the physical gates pass, release language is limited to “serial DMX
output implemented; USB-DMX hardware compatibility and fixture acceptance
pending.” The current show transport boundary is USB-DMX primary plus the
simultaneous local Art-Net Unity mirror; historical records that described
serial DMX as outside the show are superseded.
