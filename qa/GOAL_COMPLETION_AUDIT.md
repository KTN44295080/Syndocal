# Production-Quality Goal Completion Audit

Updated: 2026-07-17 (addendum below; base table rows dated 2026-07-15)
Scope: Syndocal as a unified lighting and VJ application, preserving `.sdc v1` and the one-screen desk.

## Verdict

The implementable software-only scope is feature-complete and locally gated. Windows cross-application NDI and Spout, a physical DirectShow camera, and physical MIDI port opening now also have host evidence. It is not honest to call the product universally perfect until the remaining rows below are captured with target lighting hardware, operator-driven controllers/remotes, representative media, two show computers, projector rigs, other operating systems, and release signing credentials.

| Goal area | Software state | Reproducible evidence | Remaining external acceptance |
|---|---|---|---|
| Operational safety | Complete locally | LAN opt-in and pairing, remote limits, destructive-action confirmation, atomic Recovery/backups, diagnostics ZIP, panic log | Tablet/phone Wi-Fi latency; production TLS/VPN boundary |
| Time and output continuity | Complete locally; MIDI ports physically opened | Shared clock source, MTC/LTC/Link timeline sync, LOCK/STALE age display, DMX route exponential reconnect telemetry/tests; 256-route configuration with 128 enabled Art-Net universe tick and 128-route viewport evidence; `SMC-Mixer-bt` input/output opened and feedback sent | Operator-driven MIDI note/CC/clock/MTC, physical LTC, and one-hour 128-universe DMX node/interface packet-loss and disconnect capture |
| Editing and UI | Complete locally; final operator sign-off open | Transaction Undo/Redo, revisioned snapshot delta, 90/100/110% scale, keyboard/focus gates, saved layout, English/Japanese 100% static coverage; browser and native F11 1920x1080 are the primary visual gates, while Windows maximized work area 1920x1032 / app client 1920x1009 and 2048x1152 ceiling plus 1366x768/1280x720 fallback containment cover the full-window Lighting FX/VJ desks and persistent Touch safety deck | Operator rehearsal with the final console/controller layout |
| Large-show UI | Complete locally | 2,000-fixture real-browser test keeps nine Mapping Fixture rows in DOM and reaches the final row; Cue/Clip/Layer/Output/DMX rendering is bounded | Venue-scale show-file rehearsal and operator sign-off |
| Lighting Cue Engine v2 | Complete locally; Daslight FX parity open | Cue Lists/Executors, tracking/block, Parts, IFCB Fade/Delay, Follow, MIB, HTP/LTP, Programmer/Blind, reference Palettes, ordered FX stack and seven-family/13-preset library; independent Color engine with 2-8 stops, four algorithms, RGB/HSV interpolation, spread and RGB/RGBW/CMY/HSV/wheel bindings; independent Chaser with ordered fixture/group/gap steps, 1-16 feature ranges, Forward/Reverse/Bounce/seeded Random, Wings, active width, duty/fading, Size/Phase/spread and free/beat clocks; independent Move with paired Pan/Tilt point paths, line/smooth interpolation, transforms, Absolute/Relative coordinates, direction, phase/spread and free/beat clocks; independent Value envelope and Curve cubic-channel-function runtimes/editors; source-linked Scene Blocks resolve the latest Cue body with 1-256 loops, optional jump targets, legacy point compatibility and publication rollback; selective Effect ON/OFF Cue Recall and scoped Look update; DMX Input/Merge, ENTTEC Pro-compatible/DMXKing/Open DMX serial output and RDM protocol tests | Per-Cue Effect parameter morph/fade; calibrated Amber/Lime/UV/multi-emitter colorimetry and physical Color/Chaser/Move/Curve acceptance; independent fixture-order/image-mapping engines; Scene Block operator rehearsal with external timecode and a physical show rig; Art-Net/sACN/serial waveform, QLC+-class serial VID/PID and dual-port expansion, RDM fixtures/gateway, controller round trip |
| VJ workflow | Operational on Windows software paths; competitive parity remains open | Dedicated full-window Clips/Outputs/Layers desk with serialized 10 fps Program + 5 fps Preview monitors over raw binary IPC; explicit P staging into a runtime-only local File/Still Preview with independent pause/seek/reverse/loop/speed, dedicated prefetch-zero renderer, stale-frame generation checks and staged playhead/speed transfer through Cut/Take; one-action first-run import creates stopped layers and an Off/Blackout Program output as one publication barrier/Undo transaction with rollback on publication contention; Libav and CLI RGBA working sets are capped to eight LRU layers, Libav sequential sessions expose open/reset/continue/reuse/eviction/error counters and pass B-frame/offset-start oracle, seek and EOF tests; Clip Grid/Deck A-B, audio/FFT, camera/screen workers, recording, NDI/Spout, 14 compiled built-in ISF FX plus ISF safe import subset, HAP R 4K GPU, projection blend/mask; NDI Test Patterns/Studio Monitor input-output and resize pass; URL To Spout input/reconnect and TouchDesigner output/resize/reconnect pass; physical camera two-cycle capture pass; production recording command 30-minute flash/click mux with 0.0 ms first/last stream drift | Final native-swapchain observation, Preview audio and live-source staging semantics, real-media monitor overhead and shared scaler/decode fan-out, 100+ effect breadth and multi-FX stack, arbitrary node editing, content browser/richer reusable templates, resizable VJ desk, camera unplug/replug, representative encoded HAP/ISF packs, projector rig, live decoder/monitor/encoder sample-clock rehearsal, macOS/Linux host runs |
| Syphon | Explicitly unavailable on current host | Route/schema and unavailable diagnostics remain functional | macOS host plus a wgpu-compatible Syphon/Metal implementation; current `syphon-wgpu 0.3` requires wgpu 29 while Syndocal uses wgpu 25 |
| Active/Standby | Software safety complete | Atomic generations, integrity fallback, output disarm, monotonic heartbeat, split-brain refusal tests | Two-computer shared-storage failover, physical fencing, duplicate-frame and recovery timing capture |
| Performance/reliability | Complete for established local gates; representative-media venue soak open | Three one-hour release soaks (base, live-FFT, mixed Color/Chaser/Move), zero drops/send failures, p99 timing budgets pass; release-build mixed Color/Chaser/Move per-tick benchmark (200 fixtures × 64 effects) at p95 ~3.0 ms / p99 ~3.4 ms / max ~4.1 ms, gated release-only at p95 ≤ 5 ms / p99 ≤ 8 ms; 4K HAP R GPU and engine large-show tests pass; deterministic 200 fixtures × 64 active effect regressions cover Color, Chaser and Move, with one Chaser level or paired Move value evaluation per fixture/effect/tick; maximum-control Chaser also evaluates 10 effects × 16 features × 64 fixtures × maximum width/Wings over 10 frames inside the focused budget | Representative multi-layer 4K media and mixed ISF/HAP one-hour venue-host soak |
| Distribution/update | Implementation complete | Cross-platform package CI, install smoke, signed-updater validation boundary, backup-before-update | HTTPS release endpoint, minisign private key, Authenticode, Apple Developer ID/notarization, real N-to-N+1 install |

## Addendum 2026-07-17: Unified desk and lighting show model (Daslight-parity series)

Landed since the 2026-07-15 table, all with full viewport-matrix and focused-test evidence
(no rows above are upgraded by this addendum until re-audited end-to-end):

- T10 unified workspace shell (e0e2e3e): persistent lower half (GROUPS + 2D stage +
  SELECTIONS + context pane) pixel-invariant across Setup/Control/Touch (0px rect delta,
  5 viewports); layout-oscillation class fixed via fixed-height + `contain: size layout`.
- T8 timeline pane expand toggle (41aaaaf): Daslight-style full-width pane with exact
  (0.01px) restore, gated into persistent-band invariance checks.
- F1 timeline layers v3 (63a9ce1): N user layers with engine-enforced mute/solo/lock,
  typed kinds (Audio/Lighting/Video), deterministic top-layer-wins dispatch; .sdc v1
  additive, legacy files byte-compatible.
- F3 conform-to-tempo (09fc18c): authored_beats, beat-domain placement, loop_fill,
  immediate BPM re-conform on the command drain (including during Play), [N.NNx] badge.
- F2 layered timeline desk (f4a62b2): the core Daslight workflow - drag a scene onto a
  layer; typed sections, functional lane gutters, kind-validated drops, edge-resize;
  live CDP drag-drop evidence; overview node budget held (3057/3500).
- F4 cue-owned FX parameters (b8de22d): EffectParamsSnapshot copy-on-capture,
  activation-scoped effect instances per block with conform rate threaded into all six
  evaluators, zero 44Hz-tick additions; 18 new tests. This closes part of the
  "Per-Cue Effect parameter" remaining item in the Lighting Cue Engine row - parameter
  MORPH/FADE between cues remains open.
- Verification infrastructure (2532943): matrix stall watchdog, Codex job watcher,
  CDP freeze autopsy (after a diagnosed silent-hang incident: stale-port half-attach +
  long-lived headless renderer freeze, both now structurally mitigated in the harness);
  executable operation-count contract (`qa/harnesses/check-operation-counts.mjs`).

Honest operability standing vs Daslight 5 (executable Syndocal side + manually observed
Daslight side; see qa/harnesses/README.md): the 2026-07-23 T18 rerun uses the current
always-visible Scene Matrix and places a Scene Block in 1 drag, matching Daslight's
observed 1 drag. Layer mute and pane expand are also at parity. The executable Syndocal
contract now covers 13 tasks, but Daslight has matched counts for only those three;
the other ten are explicitly unmeasured. No operability-superiority or broad-parity
claim is made yet.

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
- `target/qa/m5-soak-mixed-lighting-3600.json`: one hour with five active effects (LFO + PositionWave + independent Color + Chaser + Move), 108,001 nonblank frames, zero drops and send failures, jitter p99 512us, queue p99 48us, command-to-DMX p99 53us, 216,004 Art-Net sends.
- `mixed_color_chaser_move_curve_release_stack_meets_44hz_budget`: release-build per-tick benchmark, 200 fixtures × 64 mixed effects including Curve, p95 2.778ms / p99 3.400ms / max 4.056ms on the 2026-07-23 run.
- `target/qa/long-av-sync/long-av-sync-30m.json`: production recording mux, 54,000 frames/30 minutes, video/audio start and end drift 0.0ms, first/last flash-click and midpoint black/silence checks pass.
- `gpu_compositor_renders_4k_hap_r_bc7_frame`: real-GPU 4K compressed HAP R path.
- `large_show_loads_200_fixtures_across_8_universes_and_100_cues`: 20,000 Cue targets across eight universes.

## Release boundary

Software loopback is not substituted for hardware acceptance. A release candidate may be produced from this state, but production sign-off must attach measurements to every applicable pending row in `qa/M4_IO_VALIDATION.md`, the Active/Standby capture in `qa/M5_RELIABILITY_VALIDATION.md`, and the signing/update runbook in `qa/UPDATE_RELEASE_RUNBOOK.md`.
