# Production-Quality Goal Completion Audit

Updated: 2026-08-09 (current operator audit; older dated addenda retained below)
Scope: Syndocal as a unified lighting and VJ application, preserving `.sdc v1` and the one-screen desk. Internal 3D is excluded; Art-Net to an external visualizer is the formal visualization boundary.

## Verdict

**The declared Daslight-comparable PC lighting-software goal is complete.** T18-T23 close the measured operation baseline, seven independent FX families and Cue transitions, complete Scene Live/Live Mixer controls, fixture onboarding, named detachable workspaces, full/partial operator lock, current native Shinkan data and a separate-process Art-Net receiver. This is a software/product-boundary PASS, not a claim of full Daslight 5 equivalence or superiority. Physical nodes/fixtures/controllers, a second-PC commercial visualizer, live GDTF Share account, hardware standalone playback, venue soak and operator sign-off remain explicitly external.

| Goal area | Software state | Reproducible evidence | Remaining external acceptance |
|---|---|---|---|
| Operational safety | Complete locally | LAN opt-in and pairing, remote limits, destructive-action confirmation, atomic Recovery/backups, diagnostics ZIP, panic log | Tablet/phone Wi-Fi latency; production TLS/VPN boundary |
| Time and output continuity | Complete locally; MIDI ports physically opened | Shared clock source, MTC/LTC/Link timeline sync, LOCK/STALE age display, DMX route exponential reconnect telemetry/tests; 256-route configuration with 128 enabled Art-Net universe tick and 128-route viewport evidence; native Shinkan separate-process U0 capture at 44 fps with 0 rejected frames and 25 ms max gap; `SMC-Mixer-bt` input/output opened and feedback sent; global MIDI/OSC visual Learn creates Cue mappings from the next input and automatically starts the listener | Operator-driven MIDI note/CC/clock/MTC and visual-Learn latency/feedback comparison, physical LTC, and one-hour 128-universe physical-node packet-loss/disconnect capture |
| Editing and UI | Complete locally; final operator sign-off open | Transaction Undo/Redo, revisioned snapshot delta, 90/100/110% scale, keyboard/focus gates, named seven-pane detachable workspaces, multiple-window/monitor restore, full/partial operator lock, English/Japanese 100% static coverage; browser/native primary and compact containment gates | Two-monitor/operator rehearsal with the final console/controller layout |
| Large-show UI | Complete locally | 2,000-fixture real-browser test keeps nine Mapping Fixture rows in DOM and reaches the final row; Cue/Clip/Layer/Output/DMX rendering is bounded | Venue-scale show-file rehearsal and operator sign-off |
| Lighting Cue Engine v2 | Complete locally; competitor breadth/field proof open | Cue Lists/Executors, tracking/block, Parts, IFCB Fade/Delay, Follow, MIB, HTP/LTP, Programmer/Blind, reference Palettes, ordered seven-family FX stack; Cue-owned same-ID fade; calibrated 3–16-emitter mixing; Scene Live speed/size/phase/direction/segment/flash; group dimmer/strobe/solo; source-linked Scene Blocks; GDTF Share/cache/repair/common pack; DMX Input/Merge, serial output and RDM tests. Native Shinkan dynamic FX and Scene Live changed separate-process ArtDMX | Physical/spectral FX acceptance; Scene Block operator rehearsal with external timecode and a physical show rig; Art-Net/sACN/serial waveform, RDM fixtures/gateway and controller round trip |
| VJ workflow | Operational on Windows software paths; competitive parity remains open | Dedicated full-window Clips/Outputs/Layers desk with serialized 10 fps Program + 5 fps Preview monitors over raw binary IPC; explicit P staging into a runtime-only local File/Still Preview with independent pause/seek/reverse/loop/speed, dedicated prefetch-zero renderer, stale-frame generation checks and staged playhead/speed transfer through Cut/Take; one-action first-run import creates stopped layers and an Off/Blackout Program output as one publication barrier/Undo transaction with rollback on publication contention; Libav and CLI RGBA working sets are capped to eight LRU layers, Libav sequential sessions expose open/reset/continue/reuse/eviction/error counters and pass B-frame/offset-start oracle, seek and EOF tests; Clip Grid/Deck A-B, audio/FFT, camera/screen workers, recording, NDI/Spout, 14 compiled built-in ISF FX plus ISF safe import subset, HAP R 4K GPU, projection blend/mask; NDI Test Patterns/Studio Monitor input-output and resize pass; URL To Spout input/reconnect and TouchDesigner output/resize/reconnect pass; physical camera two-cycle capture pass; production recording command 30-minute flash/click mux with 0.0 ms first/last stream drift | Final native-swapchain observation, Preview audio and live-source staging semantics, real-media monitor overhead and shared scaler/decode fan-out, 100+ effect breadth and multi-FX stack, arbitrary node editing, content browser/richer reusable templates, resizable VJ desk, camera unplug/replug, representative encoded HAP/ISF packs, projector rig, live decoder/monitor/encoder sample-clock rehearsal, macOS/Linux host runs |
| Syphon | Explicitly unavailable on current host | Route/schema and unavailable diagnostics remain functional | macOS host plus a wgpu-compatible Syphon/Metal implementation; current `syphon-wgpu 0.3` requires wgpu 29 while Syndocal uses wgpu 25 |
| Active/Standby | Software safety complete | Atomic generations, integrity fallback, output disarm, monotonic heartbeat, split-brain refusal tests | Two-computer shared-storage failover, physical fencing, duplicate-frame and recovery timing capture |
| Performance/reliability | Software budget complete; venue soaks open | Three one-hour release soaks pass. Current-head mixed 200-fixture × 64-effect release gate passes at p95 3.801 / p99 4.398 / max 5.291 ms; all-Cue-transition load passes at 11.378 / 12.858 / 12.994 ms. Calibrated RGB/Amber/Lime stack passes at 7.672 / 8.052 / 9.660 ms and calibrated all-transition stress at 17.489 / 18.119 / 18.390 ms. Native Shinkan external-process capture sustained 44 fps with 25 ms max gap | Representative multi-layer media/mixed ISF-HAP venue soak and sustained physical Art-Net-node capture |
| Distribution/update | Implementation complete | Cross-platform package CI, install smoke, signed-updater validation boundary, backup-before-update | HTTPS release endpoint, minisign private key, Authenticode, Apple Developer ID/notarization, real N-to-N+1 install |

## Addendum 2026-07-23: Daslight-comparable lighting software completion

- T18: operation-count harness now executes 13 production gestures and validates results. All pass; the three Daslight-measured tasks are equal, while ten remain explicitly unmeasured.
- T19: independent Curve, fixture-order Mapping and embedded 2D Colour Mapping, Cue-owned Effect transitions and calibrated multi-emitter mixing complete the seven-family engine structure.
- T20: Scene Live direction/segment and Live Mixer group strobe/solo complete the direct live-operation surface without expanding the 44 Hz lookup/allocation path.
- T21: faceted GDTF Share onboarding, favorites, validated offline cache/health, exact-layout repair and a common-rig pack close the software onboarding workflow without persisting credentials.
- T22 (`83970ea`): seven-pane named detachable workspaces, multiple-window/monitor restoration and credential-safe full/partial operator lock passed full Rust, 232 browser viewport cases, native seven-pane acceptance, localization and build.
- T23: native WebView2 imported Shinkan2026 as 41 fixtures / 13 banks / 30 cues. `1.1 新宝島`, dynamic `2.1 Fl-StrobeChase`, Scene Live x4 and re-trigger x1 were operated against an independent Art-Net receiver. The final 20-second artifact records 880 valid changing U0 512-byte ArtDMX frames, 0 rejected frames, 0 per-stream sequence discontinuities and 25 ms maximum gap. Full Rust, five-size full viewport, focused Scene Live 5/5, 2802/2802 localization, 506.33 kB production build and the 13-task operation harness are green.
- IP boundary: same-host send to the machine's own `192.168.1.34` adapter address did not return to local receivers; `127.0.0.1` immediately did. This is recorded as a same-NIC return/bind/firewall distinction. A real receiver uses the external PC/node LAN address and remains a physical acceptance gate.

## Addendum 2026-08-09: Core operator evidence

- The current operation corpus is 17 tasks: 15 production CDP gestures plus Save and named-project reopen.
- Sixteen have direct same-task Daslight observations: fifteen are equal and Touch control creation is one click shorter in Syndocal. Scene Live modifier bulk reset has no one-action Daslight equivalent.
- Daslight Save is one `Ctrl+S` gesture and named reopen is `Ctrl+O`, path, `Enter` (three operations), observed in the maximized desktop app against a disposable scratch `.dvc`.
- Syndocal native Save/Open wrote and reread a disposable `.sdc`, restoring its Touch surface. `app/scripts/check-project-shortcuts.mjs` executes the exact production dispatcher and fixes `Ctrl+S` and `Ctrl+O` at one key gesture each; direct modifier-key injection into WebView2 was unavailable to Computer Use and is not represented as native observation.
- Patch is one click from a prepared profile/address in both products. Static Dimmer Full is one click from a prepared EDIT scene in both products. Mapping FX creation plus prepared target inheritance is one click in both products; Daslight visibly reported `Selected beams / 4 Beam(s)` and Syndocal reopened the new `ColorMapping` with `fixture:1`.

## Addendum 2026-08-09: Visual MIDI/OSC/DMX Learn

- Full-size MIDI, OSC and DMX Learn controls now live in the shared topbar. Learn mode colors all supported visible controls purple without changing their geometry; selecting a target adds an opacity-only dashed pulse that honors reduced-motion and pauses while off-screen.
- The selected scene/fixture/master/blackout/timeline/Touch target is not executed while Learn is active. The next MIDI note/CC, OSC address or changed DMX channel creates the production mapping, replaces an existing binding from the same source, and starts or restarts the listener automatically. Timeout or failure leaves the prior mapping intact.
- The existing detailed MIDI/OSC editors, feedback, `.midimap` / `.oscmap` files and user-template persistence remain available. Video Master was added to both protocols and has direct input/feedback tests.
- Learned mappings are project data: normal `.sdc` Save/Open, browser Recovery checkpoints, desktop autosave backups, pre-update backups and user templates preserve all three mapping sets. Empty legacy `.sdc` files retain their prior serialized shape and load with empty mappings; New clears mappings, while DVC import clears the previous show and installs verified MIDI and DMX shortcuts recovered from the imported show.
- `check:live-desk-header` and `--setup-io-only` pass at 1920x1080, 1920x1032, 2048x1152, 1366x768 and 1280x720 while proving the 42px single-row topbar, three 32x40 Learn controls, visible-target arming, dashed selection, Cue mapping creation, listener restart and Setup reachability.

## Addendum 2026-08-09: DVC MIDI shortcut preservation

- Daslight `SHORTCUT TYPE=1` input tuples are parsed fail-closed as status/channel/number/learned-value/device. Verified action 107 restores Scene Play and action 55 restores Tap Tempo. Embedded Daslight action-table evidence identifies 108/109/110 as directional Scene Play and 113 as Bank Next; imported Banks are now distinct Cue Lists, direction survives Cue pre-wait, and Homecoming restores all 17/17 mappings. Action 229 is preserved as a visible Feature-fader index: the UI publishes its ordered runtime selection, while Tauri, MIDI, OSC and Remote/AI inputs all reuse one fail-closed backend resolver and the existing `SetFixtureAttributeBatch` engine route. The Laser golden restores both 229 mappings (CC8/9 -> visible faders 1/2) without guessing a fixed attribute.
- Action 107 with `FLASH=1` persists as `FlashCue`: Note/CC press triggers the imported cue and release reuses the engine's existing explicit `ReleaseCue` route. Manual Note Off discrete mappings now also fire instead of being accepted by the editor but ignored at runtime.
- The import result carries its MIDI mappings into the production controller state and subsequent `.sdc` Save/Recovery/backup path. Daslight `OUT` / `OUT1` / `OUT2` feedback is now preserved as exact OFF / ON / Unknown message type, output channel, number and velocity/value; continuous mappings interpolate those endpoints, and discrete mappings follow live state. Parallel Cue List/group activity is part of the active-state calculation. The same three states can be authored in the normal mapping editor. Only MIDI input/output device affinity remains a Setup > I/O choice, with physical controller LED color, latency and feel left to external acceptance.
- Current gates prove MIDI/OSC input, shared Engine batch output, custom OFF/ON/Unknown output, continuous interpolation, parallel-cue feedback, mixed-value handling, validation and `.sdc`/backup persistence. After the backend auto-feedback tranche, I/O is 112/112 with one physical MIDI test ignored, Engine is 454/454 with two manual benchmarks ignored, desktop is 371/371 with nine physical/long-running tests ignored, the DVC MIDI source gate is 39/39, the backend operator contract inventories 364 commands / 204 literal frontend calls / 162 transactional mutations, localization is 2979/2979, and the production web build passes. Existing five-viewport Setup I/O gates remain part of the release matrix.

### Auto feedback latency and duplicate suppression

- Auto feedback is owned by a named backend worker rather than a 500 ms frontend send timer. It samples the already-published Engine snapshot at the same 44 Hz cadence as DMX, so Timeline position, fades, masters, submasters, Cue state and continuous selected-feature values do not wait for the visible UI snapshot poll.
- The MIDI output cache retains mapping-slot identity, collapses duplicate hardware destinations to the last desired slot, and sends only slots whose complete message changed. Slot identity is required because DVC OFF/ON/Unknown states may intentionally use different message types, channels or numbers; returning to an earlier state must still resend it. Reconfiguring mappings restarts the worker and forces one complete current-state refresh; the explicit `Send Feedback` action also forces a refresh.
- `set_midi_feedback_auto`, `midi_feedback_status` and `send_midi_feedback` are typed backend commands. A worker send/lock fault is retained as status, stops the worker and causes the frontend health check to clear the Auto toggle instead of silently claiming feedback is active.
- This proves scheduling, state resolution and suppression without MIDI hardware. USB/MIDI driver buffering, controller LED palette, motor response and measured end-to-end latency remain physical acceptance boundaries.
- Native `pnpm --dir app tauri build --no-bundle` passed after the exact-checkout process guard reported zero running instances. The 2026-08-09 15:31:03+09:00 release executable is 38,871,040 bytes with SHA-256 `8C16E333DEBBDC499FF7118285C22AA984A703BFDD69C346EBB9E82F6DA6CE37`. It was not launched because another task owned the desktop surface.

## Addendum 2026-08-09: DVC DMX Control Mapping preservation

- Daslight `SHORTCUT TYPE=3` is now parsed fail-closed. Only the verified `/dmx/<universe>/<channel>:5`, action 210, exact scalar settings, matching profile UID/raw feature index and primary-beam target shape converts; unproven variants remain Skipped/Unsupported.
- Read-only `Panel.dvc` restores 9/9 U1 RGB control routes exactly, including its authored third-fixture Blue/Red/Green order, with zero skipped and zero unsupported mappings. A synthetic negative specimen fixes unknown selector/action/settings behavior.
- Setup > I/O separates Raw Merge from Control Mapping so one source is never applied twice. Streaming dispatch is change-driven; a held channel cannot retrigger every frame, while signal loss re-arms it. A real UDP loopback proves two identical ArtDMX frames produce exactly one mapped event.
- The typed mapping uses the complete OSC action surface and shared backend dispatcher. `.sdc`, Recovery, backup and user-template round trips pass and every non-empty restore re-enters Control Mapping mode. Current full gates are protocol 43/43, I/O 108/108 plus one physical MIDI ignore, desktop 369/369 plus nine physical/long-running ignores, the 33-assertion DMX source gate, 2964/2964 localization and both focused five-viewport suites. Physical DMX interface/node/lamp response and venue latency remain external acceptance.

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
- `mixed_color_chaser_move_curve_mapping_and_color_mapping_release_stack_meets_44hz_budget`: current-head release-build per-tick benchmark, 200 fixtures × 64 mixed effects across the production stack, p95 3.801ms / p99 4.398ms / max 5.291ms on the T23 final run. Its deliberate all-64-transition companion measured p95 11.378ms / p99 12.858ms / max 12.994ms. Both passed their fixed gates.
- `calibrated_multi_emitter_mixed_release_stack_stays_inside_44hz_tick`: release-build calibrated 200-fixture × 64-effect production mix, p95 7.672ms / p99 8.052ms / max 9.660ms; deliberate all-transition stress p95 17.489ms / p99 18.119ms / max 18.390ms. Both passed their dedicated gates. `qa/CALIBRATED_MULTI_EMITTER_ACCEPTANCE.md` records the separate marginally-red ordinary same-host A/B without relabeling it green.
- `target/qa/long-av-sync/long-av-sync-30m.json`: production recording mux, 54,000 frames/30 minutes, video/audio start and end drift 0.0ms, first/last flash-click and midpoint black/silence checks pass.
- `gpu_compositor_renders_4k_hap_r_bc7_frame`: real-GPU 4K compressed HAP R path.
- `large_show_loads_200_fixtures_across_8_universes_and_100_cues`: 20,000 Cue targets across eight universes.

## Release boundary

The separate-process loopback closes the Syndocal software/ArtDMX boundary; it is not substituted for hardware acceptance. The Daslight-comparable PC software goal is complete and a release candidate may be produced from this state. Production/venue sign-off must still attach measurements to every applicable pending row in `qa/M4_IO_VALIDATION.md`, the Active/Standby capture in `qa/M5_RELIABILITY_VALIDATION.md`, and the signing/update runbook in `qa/UPDATE_RELEASE_RUNBOOK.md`.
