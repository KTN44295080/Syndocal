# M4 External I/O Validation Matrix

Updated: 2026-09-07

This is an engineering evidence record, not user documentation. README and release documentation remain deferred to M6.

## Current host check — 2026-09-07

The current release candidate was built and launched from the exact checkout.
The native window was responsive and maximized, and no physical output route was
enabled during the check. `cargo test -p io --locked -- --test-threads=1` passed
`180 / 0 / 2` (passed / failed / ignored). The safe device inventory found ASUS
5M, ASUS 13M AF, and ASUS IR cameras, plus virtual MIDI ports; no serial DMX/
Enttec/COM device and no physical MIDI endpoint was present.

The ASUS DirectShow camera was exercised through the production capture worker at
1280x720 NV12/30fps. Two clean start/stop cycles passed (`1/1`). This updates the
current camera evidence only; it does not replace venue capture, unplug/replug,
physical MIDI interaction, DMX fixture output, or integrated show acceptance.
Those rows remain external hardware gates. Existing historical FT232R/COM3,
SMC-Mixer, NDI, and Spout evidence remains valid for its recorded device/date and
must not be inferred as a current-host comprehensive acceptance.

| Path | Automated or local evidence | Physical hardware evidence | Status |
|---|---|---|---|
| Art-Net output | `io::artnet::tests::sends_art_dmx_to_udp_loopback` and Tauri multi-route loopback pass | Art-Net node -> fixture output capture required | Software pass; hardware pending |
| sACN / E1.31 output | Packet, multicast address derivation, and UDP loopback tests pass | Multicast-capable node -> fixture output capture required | Software pass; hardware pending |
| Enttec USB Pro / DMXKing | Packet framing and serial route validation tests pass | USB interface + DMX receiver capture required | Software pass; hardware pending |
| Enttec Open DMX | Dedicated worker, bounded latest-frame mailbox, 176 us break, 16 us MAB, payload, and frame-pacing tests pass (11/11) | 2026-08-03 physical pass: generic FT232R USB-RS485 cable (VID 0403 / PID 6001, COM3) drove a 4ch Dimmer/RGB fixture at address 1 with sustained white and dimmed output, operator-confirmed stable. The rig exposed a real defect fixed the same day: the worker loop had no wire-rate pacing, so each frame's break sliced the previous frame still draining from the FTDI buffer (symptom: one initial flash then dark; with a 2 ms guard, a ~2 s periodic dropout). Pacing to the 22,764 us frame wire time plus an empirically required 8 ms guard (~32 fps) stabilized output. 2026-08-04 second physical pass at scale: the fixture reconfigured to its 121ch mode (ch1 master dimmer + 40x RGB cells) was driven through the same COM3 path by the env-gated `physical_serial_rainbow_demo_drives_master_dimmer_and_rgb_cells` engine test (three phase-shifted PositionWave effects forming a moving rainbow; 60.2 s, 2,641+ successful serial sends, zero failures) and the operator visually confirmed the moving rainbow across the cells. Logic-analyzer break/MAB waveform capture still pending (no analyzer on site) | Physical pass (4ch + 121ch); waveform pending |
| MIDI input/output | Decode, clock, MTC, mapping, and feedback tests pass | `SMC-Mixer-bt` input/output enumerated and opened through the production `midir` path; a safe All Notes Off feedback message was sent. Physical note/CC/clock movement and visible feedback confirmation remain | Partial physical pass; operator interaction pending |
| OSC input | Mapping, wildcard, clock, effect, video, and bundle tests pass | TouchOSC or equivalent LAN round trip recommended | Software pass; device pending |
| Web remote | HTTP/WebSocket protocol and snapshot/status tests pass | iPad/Android Wi-Fi interaction and latency capture required | Software pass; device pending |
| NDI input/output | Official Windows NDI 6 SDK feature build; local sender -> discovery -> RGBA receiver loopback passes. External NDI Test Patterns -> Syndocal input received 1920x1080 RGBA at 30000/1001. Syndocal output -> NDI Studio Monitor displayed the QA bars, reported 1-2 SDK clients, and stayed connected across 640x360 -> 1280x720 | Representative OBS/Resolume material and macOS/Linux host coverage remain recommended | Windows NDI 6 cross-application pass |
| Spout input/output | Default Windows x86_64 build statically links Spout2 2.007.017. `URL To SpoutSyphon` -> Syndocal input passed at 1920x1080 DXGI format 28 and recovered after the external sender was closed/relaunched. Syndocal output -> TouchDesigner 2023.12000 displayed RGBA bars, auto-reconnected after sender restart, and updated from 640x360 to 1280x720 | Representative show material and a sustained dropped-frame capture remain recommended | Windows cross-application input/output, reconnect, and resize pass |
| Camera / screen capture | Persistent FFmpeg worker uses DirectShow/gdigrab on Windows, AVFoundation on macOS, and V4L2/X11Grab on Linux. Physical `nuroum Webcam V15AF` produced two clean 1280x720 RGBA start/stop cycles through the production worker. DirectShow reported MJPEG up to 2560x1440/60 and YUYV modes | Physical unplug/replug and macOS/Linux device/display capture remain | Windows physical camera selection/format/restart pass; unplug and other hosts pending |
| Syphon input/output | Route/schema/platform diagnostics exist; current `syphon-wgpu 0.3` requires wgpu 29 while Syndocal is pinned to wgpu 25, and `Syphon.framework` is unavailable on the Windows development host | macOS host with matching Syphon.framework and Metal/wgpu integration required | Explicitly unavailable; macOS implementation pending |
| RDM Art-Net / USB Pro | E1.20, ArtRdm/TOD, USB Pro Label 5/7/11, ACK_OVERFLOW, ACK_TIMER, queued message, discovery splitting, timeout and collision-safe parser tests pass | Art-Net gateway and USB Pro with at least two RDM fixtures, including collision discovery and inventory churn, required | Software pass; hardware pending |
| VJ recording A/V | Command graph tests cover output-scoped source selection, seek, loop, gain, 0.25-4x `atempo`, `amix`, AAC mux, and silent fallback. The production command recorded a deterministic 30-minute/54,000-frame H.264/AAC MP4 with synchronized white-flash/1kHz-click events at both ends. FFprobe measured video/audio start and duration as 0.000/1800.000 seconds, for 0.0 ms first/last drift; decoded first/last events and black/silent midpoint checks passed | Representative show media through the live decoder, monitor device, and recording path still requires venue-host/operator capture because those components do not share one sample clock | 30-minute local production-mux sync pass; live venue path pending |
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

# External NDI sender/receiver applications.
$env:SYNDOCAL_TEST_NDI_SOURCE = "Test Pattern"
cargo test -p io --features ndi --locked receives_rgba_from_an_external_ndi_application -- --ignored --nocapture
$env:SYNDOCAL_TEST_NDI_OUTPUT = "Syndocal External QA"
cargo test -p io --features ndi --locked publishes_rgba_to_an_external_ndi_application_and_survives_resize -- --ignored --nocapture

# Windows x86_64 Spout2: the two tests below use the real GPU transport.
cargo test -p syndocal --locked input_worker_receives_a_real_spout_frame -- --ignored --nocapture
cargo test -p syndocal --locked output_worker_publishes_the_engine_composition -- --ignored --nocapture
$env:SYNDOCAL_TEST_SPOUT_SENDER = "0UtS-Spout"
cargo test -p syndocal --locked input_worker_receives_from_an_external_spout_application -- --ignored --nocapture
cargo test -p syndocal --locked input_worker_recovers_when_external_spout_sender_restarts -- --ignored --nocapture
$env:SYNDOCAL_TEST_SPOUT_OUTPUT = "Syndocal External QA"
$env:SYNDOCAL_TEST_SPOUT_CONFIRM_FILE = "C:\path\to\operator-confirm.txt"
cargo test -p syndocal --locked output_worker_publishes_to_an_external_spout_application_and_survives_resize -- --ignored --nocapture

# Physical DirectShow camera and physical MIDI ports.
$env:SYNDOCAL_FFMPEG = "C:\path\to\ffmpeg.exe"
$env:SYNDOCAL_TEST_CAMERA_ENDPOINT = "nuroum Webcam V15AF"
cargo test -p syndocal --locked camera_worker_captures_a_real_frame_and_restarts_cleanly -- --ignored --nocapture
$env:SYNDOCAL_TEST_MIDI_INPUT = "SMC-Mixer"
$env:SYNDOCAL_TEST_MIDI_OUTPUT = "SMC-Mixer"
cargo test -p io --locked physical_midi_ports_enumerate_open_and_send_feedback -- --ignored --nocapture

# Local FFmpeg/FFprobe H.264 + AAC recording/mux runtime.
cargo test -p syndocal --locked recording_command_writes_a_real_video_and_audio_mp4 -- --ignored --nocapture
$env:SYNDOCAL_LONG_AV_QA_DIR = "$PWD/target/qa/long-av-sync"
cargo test -p syndocal --locked recording_command_keeps_long_av_sync_with_first_and_last_flash_clicks -- --ignored --nocapture

# ISF parser/safety boundary and real wgpu render path.
cargo test -p video --locked isf_runtime -- --nocapture
cargo test -p video --locked video_preview_renderer_ -- --nocapture

# HAP R BC7 parser, CPU fallback, GPU parity, and explicit 4K GPU run.
cargo test -p video --locked hap_decoder -- --nocapture
cargo test -p video --locked gpu_compositor_matches_cpu_for_rgba_bgra_bc1_bc3_and_bc7 -- --nocapture
cargo test -p video --locked gpu_compositor_renders_4k_hap_r_bc7_frame -- --ignored --nocapture
```

## Acceptance Capture

### 2026-07-13 Windows development host

| Path | Device/application | Capture | Result |
|---|---|---|---|
| Camera | `nuroum Webcam V15AF`, FFmpeg DirectShow | Production capture worker received 1280x720 RGBA and stopped cleanly twice in 8.28 s. Advertised MJPEG modes include 1280x720, 1920x1080, and 2560x1440 at 30-60 fps; YUYV 640 modes advertise 30 fps | Pass for selection, negotiation, restart, and cleanup; physical unplug/replug not performed |
| MIDI | `SMC-Mixer-bt` physical input/output | Production `midir` enumeration and open succeeded; one safe channel-1 All Notes Off (`B0 7B 00`) feedback message sent in 0.12 s | Partial pass; no operator knob/button/clock/MTC movement was available |
| NDI input | NDI 6 Test Patterns `DESKTOP-FT3CSEN (Test Pattern)` | Production `NdiInput` discovered and received 1920x1080 RGBA at 30000/1001 in 0.52 s | Pass |
| NDI output | NDI 6 Studio Monitor | QA bars visibly received; SDK sender reported 1-2 connected clients; Studio Monitor remained connected across 640x360 -> 1280x720 at 30 fps; automated test passed in 16.50 s | Pass |
| Spout input | URL To Spout/Syphon sender `0UtS-Spout` | Production input received 1920x1080, DXGI format 28. New-frame polling was 171.7 polls/s, comfortably above the 30 fps gate but not asserted as the sender's true frame rate | Pass |
| Spout input reconnect | URL To Spout/Syphon | Sender was closed, disappearance observed, relaunched, and the same Syndocal worker resumed frames; test completed in 42.85 s | Pass |
| Spout output/resize | TouchDesigner 2023.12000 `Syphon Spout In TOP` | Syndocal composition bars visibly received as 8-bit fixed RGBA. TouchDesigner reported 640x360, then 1280x720 after live output reconfiguration; the node retained the sender name and auto-reconnected after sender restart | Pass |
| VJ recording long A/V sync | FFmpeg 7.1 / FFprobe 7.1, production `video_recording_ffmpeg_command` | 30:00.000, 54,000 frames at 30 fps; video/audio both start at 0.000 and end at 1800.000; first/last flash luma 255; first/last click peaks 4646/4133; midpoint luma/PCM peak 0; report at `target/qa/long-av-sync/long-av-sync-30m.json` | Pass |

The NDI 6 Test Patterns and Studio Monitor executables could not directly start on this host because the machine-wide .NET 7 WindowsDesktop runtimeconfig is corrupt. The source-controlled launchers in `qa/harnesses` apply `DOTNET_ROLL_FORWARD=LatestMajor` only to their child process and do not modify Program Files.

For each still-pending physical row, record the date, OS, device/software version, project file, measured latency or waveform, and pass/fail result here. Do not convert a software loopback into a physical pass.

For RDM, also record transport, gateway/interface firmware, fixture UID/PID, discovery duration, collision fixture count, ACK/NACK/timeout behavior, and inventory add/remove time. For A/V recording, attach the `ffprobe -show_streams -show_format` output and measured first/last sync error. For projection, attach the source mask, projector model, overlap width/gamma/black-level values, and before/after capture. For Spout, record sender/receiver applications, texture format, dimensions, frame rate, resize/reconnect result, and dropped-frame observation.
