# M4 External I/O Validation Matrix

Updated: 2026-07-13

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
| Spout input/output | Default Windows x86_64 build statically links Spout2 2.007.017. Real DirectX sender -> Syndocal input worker and Syndocal composition output worker -> receiver loopbacks pass on the development GPU | OBS/Resolume/TouchDesigner cross-application RGBA/BGRA, resize, reconnect, and 60fps capture required | Local GPU runtime pass; external app pending |
| Camera / screen capture | Persistent FFmpeg worker uses DirectShow/gdigrab on Windows, AVFoundation on macOS, and V4L2/X11Grab on Linux; latest 1280x720 RGBA frame is exchanged without per-frame process creation. A real Windows desktop capture reached the decoder registry and stopped cleanly | USB/HDMI capture camera selection, unplug/replug, format negotiation, and macOS/Linux device/display capture required | Windows desktop runtime pass; camera and other hosts pending |
| Syphon input/output | Route/schema/platform diagnostics exist; current `syphon-wgpu 0.3` requires wgpu 29 while Syndocal is pinned to wgpu 25, and `Syphon.framework` is unavailable on the Windows development host | macOS host with matching Syphon.framework and Metal/wgpu integration required | Explicitly unavailable; macOS implementation pending |
| RDM Art-Net / USB Pro | E1.20, ArtRdm/TOD, USB Pro Label 5/7/11, ACK_OVERFLOW, ACK_TIMER, queued message, discovery splitting, timeout and collision-safe parser tests pass | Art-Net gateway and USB Pro with at least two RDM fixtures, including collision discovery and inventory churn, required | Software pass; hardware pending |
| VJ recording A/V | Command graph tests cover output-scoped source selection, seek, loop, gain, 0.25-4x `atempo`, `amix`, AAC mux, and silent fallback. A real raw-RGBA + PCM tone run produces an H.264/AAC MP4 and passes FFprobe stream verification | 30+ minute clip with visible clap/flash reference and first/last drift measurement required | Local encoder runtime pass; long material pending |
| HAP Q Alpha / HAP R | In-process MOV parser covers dual-plane YCoCg BC3 + BC4 alpha and `Hap7` RGBA BC7. HAP R stays block-compressed for direct `Bc7RgbaUnorm` GPU sampling and uses `bcdec_rs` for CPU-visible paths. Synthetic Hap7 MOV with alpha, odd-size crop, CPU/GPU parity, fallback telemetry, and real-GPU 4K BC7 render pass | Encoder-produced HAP R/Q Alpha clips, multi-layer SSD playback, seek/reverse/loop, and one-hour dropped-frame capture required | Local parser/GPU runtime pass; real material soak pending |
| ISF shader | Embedded ISF 2 single-pass image filters are translated from bounded GLSL to validated WGSL and run through wgpu. Real GPU threshold output, prepared-shader/pipeline cache reuse, fail-open, control clamp, duplicate, `.sdc v1` round trip, legacy default, and unsafe-feature rejection tests pass | Representative ISF pack at 1080p/4K, parameter automation, GPU reset, and one-hour mixed-output soak required | Local GPU runtime pass; pack/soak pending |
| Projection blend/mask | CPU/GPU parity covers edge blend, black level, polygon mask, embedded 16x16 luma mask, inversion, quantization, and legacy/preset compatibility | Two-projector overlap, black-level match, and imported venue mask photometric capture required | Software pass; projector rig pending |
| Native Display output | M3 Windows wgpu nonblank, resize, multi-output, HAP Q, and frame-time evidence recorded in `COMPLETION_PLAN.md` | macOS/Linux display smoke remains an M6 host gate | Windows pass; other hosts pending |

## Reproduction

```powershell
cargo test -p io --locked

$env:NDI_SDK_DIR = "C:\path\to\NDI 6 SDK"
$env:PATH = "$env:NDI_SDK_DIR\Bin\x64;$env:PATH"
cargo test -p io --features ndi --locked
cargo test -p io --features ndi --locked sends_and_receives_rgba_over_local_ndi -- --ignored
cargo test -p syndocal --no-default-features --features ndi --locked

# Windows x86_64 Spout2: the two tests below use the real GPU transport.
cargo test -p syndocal --locked input_worker_receives_a_real_spout_frame -- --ignored --nocapture
cargo test -p syndocal --locked output_worker_publishes_the_engine_composition -- --ignored --nocapture

# Local FFmpeg/FFprobe H.264 + AAC recording/mux runtime.
cargo test -p syndocal --locked recording_command_writes_a_real_video_and_audio_mp4 -- --ignored --nocapture

# ISF parser/safety boundary and real wgpu render path.
cargo test -p video --locked isf_runtime -- --nocapture
cargo test -p video --locked video_preview_renderer_ -- --nocapture

# HAP R BC7 parser, CPU fallback, GPU parity, and explicit 4K GPU run.
cargo test -p video --locked hap_decoder -- --nocapture
cargo test -p video --locked gpu_compositor_matches_cpu_for_rgba_bgra_bc1_bc3_and_bc7 -- --nocapture
cargo test -p video --locked gpu_compositor_renders_4k_hap_r_bc7_frame -- --ignored --nocapture
```

## Acceptance Capture

For each pending physical row, record the date, OS, device/software version, project file, measured latency or waveform, and pass/fail result here. Do not convert a software loopback into a physical pass.

For RDM, also record transport, gateway/interface firmware, fixture UID/PID, discovery duration, collision fixture count, ACK/NACK/timeout behavior, and inventory add/remove time. For A/V recording, attach the `ffprobe -show_streams -show_format` output and measured first/last sync error. For projection, attach the source mask, projector model, overlap width/gamma/black-level values, and before/after capture. For Spout, record sender/receiver applications, texture format, dimensions, frame rate, resize/reconnect result, and dropped-frame observation.
