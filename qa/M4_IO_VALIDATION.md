# M4 External I/O Validation Matrix

Updated: 2026-07-12

This is an engineering evidence record, not user documentation. README and release documentation remain deferred to M6.

| Path | Automated or local evidence | Physical hardware evidence | Status |
|---|---|---|---|
| Art-Net output | `io::artnet::tests::sends_art_dmx_to_udp_loopback` and Tauri multi-route loopback pass | Art-Net node -> fixture output capture required | Software pass; hardware pending |
| sACN / E1.31 output | Packet, multicast address derivation, and UDP loopback tests pass | Multicast-capable node -> fixture output capture required | Software pass; hardware pending |
| Enttec USB Pro / DMXKing | Packet framing and serial route validation tests pass | USB interface + DMX receiver capture required | Software pass; hardware pending |
| Enttec Open DMX | Dedicated worker, bounded latest-frame mailbox, 176 us break, 16 us MAB, and payload tests pass | Logic-analyzer break/MAB/frame-period capture required | Software pass; waveform pending |
| MIDI input/output | Decode, clock, MTC, mapping, and feedback tests pass | Controller note/CC/clock/feedback round trip required | Software pass; hardware pending |
| OSC input | Mapping, wildcard, clock, effect, video, and bundle tests pass | TouchOSC or equivalent LAN round trip recommended | Software pass; device pending |
| Web remote | HTTP/WebSocket protocol and snapshot/status tests pass | iPad/Android Wi-Fi interaction and latency capture required | Software pass; device pending |
| NDI input/output | Official Windows NDI 6 SDK feature build; local sender -> discovery -> RGBA receiver loopback passes in 0.20 s | OBS/Resolume cross-application send and receive required | Local runtime pass; external app pending |
| Native Display output | M3 Windows wgpu nonblank, resize, multi-output, HAP Q, and frame-time evidence recorded in `COMPLETION_PLAN.md` | macOS/Linux display smoke remains an M6 host gate | Windows pass; other hosts pending |

## Reproduction

```powershell
cargo test -p io --locked

$env:NDI_SDK_DIR = "C:\path\to\NDI 6 SDK"
$env:PATH = "$env:NDI_SDK_DIR\Bin\x64;$env:PATH"
cargo test -p io --features ndi --locked
cargo test -p io --features ndi --locked sends_and_receives_rgba_over_local_ndi -- --ignored
cargo test -p syndocal --no-default-features --features ndi --locked
```

## Acceptance Capture

For each pending physical row, record the date, OS, device/software version, project file, measured latency or waveform, and pass/fail result here. Do not convert a software loopback into a physical pass.
