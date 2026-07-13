# Production-Quality Goal Completion Audit

Updated: 2026-07-13
Scope: Syndocal as a unified lighting and VJ application, preserving `.sdc v1` and the one-screen desk.

## Verdict

The implementable software-only scope is feature-complete and locally gated. Windows cross-application NDI and Spout, a physical DirectShow camera, and physical MIDI port opening now also have host evidence. It is not honest to call the product universally perfect until the remaining rows below are captured with target lighting hardware, operator-driven controllers/remotes, representative media, two show computers, projector rigs, other operating systems, and release signing credentials.

| Goal area | Software state | Reproducible evidence | Remaining external acceptance |
|---|---|---|---|
| Operational safety | Complete locally | LAN opt-in and pairing, remote limits, destructive-action confirmation, atomic Recovery/backups, diagnostics ZIP, panic log | Tablet/phone Wi-Fi latency; production TLS/VPN boundary |
| Time and output continuity | Complete locally; MIDI ports physically opened | Shared clock source, MTC/LTC/Link timeline sync, LOCK/STALE age display, DMX route exponential reconnect telemetry/tests; 256-route configuration with 128 enabled Art-Net universe tick and 128-route viewport evidence; `SMC-Mixer-bt` input/output opened and feedback sent | Operator-driven MIDI note/CC/clock/MTC, physical LTC, and one-hour 128-universe DMX node/interface packet-loss and disconnect capture |
| Editing and UI | Complete locally; final operator sign-off open | Transaction Undo/Redo, revisioned snapshot delta, 90/100/110% scale, keyboard/focus gates, saved layout, English/Japanese 100% static coverage; full-window Lighting FX/VJ desks and persistent Touch safety deck pass 1280/1366/2048 containment | Operator rehearsal with the final console/controller layout |
| Large-show UI | Complete locally | 2,000-fixture real-browser test keeps nine Mapping Fixture rows in DOM and reaches the final row; Cue/Clip/Layer/Output/DMX rendering is bounded | Venue-scale show-file rehearsal and operator sign-off |
| Lighting Cue Engine v2 | Complete locally; Daslight FX parity open | Cue Lists/Executors, tracking/block, Parts, IFCB Fade/Delay, Follow, MIB, HTP/LTP, Programmer/Blind, reference Palettes, ordered FX stack and seven-family/13-preset library; independent 2-8-stop Color engine with Cycle/Bounce/Sequence/seeded Random, RGB/HSV interpolation, deterministic spread, RGB/RGBW/CMY/HSV/colored-wheel bindings and publication-safe add/update; DMX Input/Merge, ENTTEC Pro-compatible/DMXKing/Open DMX serial output, RDM protocol tests | Cue recall for normal Effect enabled state; calibrated Amber/Lime/UV/multi-emitter colorimetry and physical color acceptance; independent chaser/move/curve/image-mapping engines; Art-Net/sACN/serial waveform, QLC+-class serial VID/PID and dual-port expansion, RDM fixtures/gateway, controller round trip |
| VJ workflow | Operational on Windows software paths; competitive parity remains open | Dedicated full-window Clips/Outputs/Layers desk with serialized 10 fps Program + 5 fps Preview monitors over raw binary IPC; explicit P staging into a runtime-only local File/Still Preview with independent pause/seek/reverse/loop/speed, dedicated prefetch-zero renderer, stale-frame generation checks and staged playhead/speed transfer through Cut/Take; one-action first-run import creates stopped layers and an Off/Blackout Program output as one publication barrier/Undo transaction with rollback on publication contention; Libav and CLI RGBA working sets are capped to eight LRU layers, Libav sequential sessions expose open/reset/continue/reuse/eviction/error counters and pass B-frame/offset-start oracle, seek and EOF tests; Clip Grid/Deck A-B, audio/FFT, camera/screen workers, recording, NDI/Spout, 14 compiled built-in ISF FX plus ISF safe import subset, HAP R 4K GPU, projection blend/mask; NDI Test Patterns/Studio Monitor input-output and resize pass; URL To Spout input/reconnect and TouchDesigner output/resize/reconnect pass; physical camera two-cycle capture pass; production recording command 30-minute flash/click mux with 0.0 ms first/last stream drift | Final native-swapchain observation, Preview audio and live-source staging semantics, real-media monitor overhead and shared scaler/decode fan-out, 100+ effect breadth and multi-FX stack, arbitrary node editing, content browser/richer reusable templates, resizable VJ desk, camera unplug/replug, representative encoded HAP/ISF packs, projector rig, live decoder/monitor/encoder sample-clock rehearsal, macOS/Linux host runs |
| Syphon | Explicitly unavailable on current host | Route/schema and unavailable diagnostics remain functional | macOS host plus a wgpu-compatible Syphon/Metal implementation; current `syphon-wgpu 0.3` requires wgpu 29 while Syndocal uses wgpu 25 |
| Active/Standby | Software safety complete | Atomic generations, integrity fallback, output disarm, monotonic heartbeat, split-brain refusal tests | Two-computer shared-storage failover, physical fencing, duplicate-frame and recovery timing capture |
| Performance/reliability | Complete for established local gates; new Color venue budget open | Two one-hour release soaks, zero drops/send failures, p99 timing budgets pass; 4K HAP R GPU and engine large-show tests pass; 200 fixtures × 64 active Color effects reuse one evaluation per fixture/effect/tick in the deterministic regression | Release-build 44 Hz/p95/p99 Color-stack benchmark and one-hour mixed Color show soak; representative multi-layer 4K media and mixed ISF/HAP one-hour venue-host soak |
| Distribution/update | Implementation complete | Cross-platform package CI, install smoke, signed-updater validation boundary, backup-before-update | HTTPS release endpoint, minisign private key, Authenticode, Apple Developer ID/notarization, real N-to-N+1 install |

## Local gates

Core commands:

```powershell
cargo test --workspace --locked --no-fail-fast
cargo check -p syndocal --no-default-features --locked
npm --prefix app run build
npm --prefix app run check:viewport
npm --prefix app run check:vj-first-run
npm --prefix app run check:vj-first-run-viewport
npm --prefix app run check:large-show-ui
npm --prefix app run check:localization
npm --prefix app run check:virtualization
npm --prefix app run check:clock-display
```

Long-running and GPU evidence:

- `target/qa/m5-soak-3600.json`: one hour, 108,001 nonblank frames, zero drops and send failures, telemetry budget pass.
- `target/qa/m5-soak-live-audio-3600.json`: one hour with 108,001 live FFT updates, zero drops and send failures, queue p99 22us, command-to-DMX p99 28us, jitter p99 823us.
- `target/qa/long-av-sync/long-av-sync-30m.json`: production recording mux, 54,000 frames/30 minutes, video/audio start and end drift 0.0ms, first/last flash-click and midpoint black/silence checks pass.
- `gpu_compositor_renders_4k_hap_r_bc7_frame`: real-GPU 4K compressed HAP R path.
- `large_show_loads_200_fixtures_across_8_universes_and_100_cues`: 20,000 Cue targets across eight universes.

## Release boundary

Software loopback is not substituted for hardware acceptance. A release candidate may be produced from this state, but production sign-off must attach measurements to every applicable pending row in `qa/M4_IO_VALIDATION.md`, the Active/Standby capture in `qa/M5_RELIABILITY_VALIDATION.md`, and the signing/update runbook in `qa/UPDATE_RELEASE_RUNBOOK.md`.
