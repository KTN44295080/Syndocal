# Serial DMX Compatibility

Updated: 2026-07-13

## Current status

Syndocal can already transmit DMX through operating-system serial ports. Setup > DMX exposes port scan/manual COM or `/dev/ttyUSB*` selection, baud rate, route apply and route test for:

- ENTTEC DMX USB Pro packet framing (`0x7e`, label 6, 513-byte DMX payload, `0xe7`)
- DMXKing ultraDMX through the compatible USB Pro packet path
- ENTTEC Open DMX and compatible VCP/FTDI serial ports at 250000 baud, 8N2, with a dedicated non-blocking worker, 176 us break and 16 us mark-after-break

Scan Serial now preserves USB VID/PID, product, manufacturer and serial number. Known single-port ENTTEC USB Pro, DMXKing ultraDMX and Open DMX product identities expose a confirmation button for the recommended protocol. Generic FTDI devices and unsupported dual-port Pro Mk2/Ultra Pro products stay on manual selection instead of receiving a potentially unsafe guess.

The engine applies the same per-route reconnect/backoff telemetry used by network output. A failed USB/serial route does not block healthy Art-Net/sACN routes.

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

1. Keep the existing three output modes as the stable serial baseline.
2. **Complete:** add USB VID/PID/product metadata and a non-destructive protocol recommendation to Scan Serial; manual override remains available. Product-matching tests and the five-reference-browser-viewport contract cover the behavior, with 1920x1080 primary, a 1920x1032 measured-client-size fixture, a 2048x1152 ceiling and 1366x768/1280x720 fallbacks.
3. Add Pro Mk2/Ultra Pro dual-output profiles using verified vendor protocol documentation and per-port routing.
4. Add supported serial DMX input modes to the existing HTP/LTP input merge path.
5. Build packet-codec tests for every profile before allowing a physical port to open.
6. Capture 512-channel waveform and one-hour output evidence for each supported device. Open DMX requires logic-analyzer break/MAB/frame-period verification; Pro-class devices require receiver capture and unplug/replug recovery.

Until the physical gates pass, release language is limited to “serial DMX output implemented; hardware compatibility pending.”
