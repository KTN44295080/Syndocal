# TouchDesigner / Auto VJ Competitive Audit

Updated: 2026-07-14

## Verdict

Syndocal is not yet equal to TouchDesigner as a general real-time visual
programming environment. It can provide a purpose-built path for a narrower
but valuable job: running a prepared lighting-and-video show with explicit
state, fail-closed behavior and deterministic recovery without constructing an
operator network at the venue.

The comparison must therefore keep two claims separate:

1. **General authoring parity:** arbitrary graph construction, GPU-resident
   texture processing, device/protocol breadth and reusable tools. This remains
   open and is a large product boundary.
2. **Prepared show / Auto VJ specialization:** a short setup path, clearer live
   state, lower operator workload, synchronized lighting/video reaction and
   deterministic recovery. These are Syndocal-specific targets that still
   require matched operator and latency evidence before any superiority claim.

## Official comparison baseline

- The comparison target is TouchDesigner's current production baseline,
  **Current Official Build 2025.32820**, published May 6, 2026. Derivative
  describes Official Builds as production-ready, tested and stable; use the
  [2025.30000 release notes](https://derivative.ca/UserGuide/Release_Notes/2025.30000)
  rather than an older 2023 build when freezing the `.toe`.
- On Windows, TouchDesigner's Audio Device In is **DirectSound/WDM or ASIO**,
  not WASAPI. It can expose mono, stereo or actual multi-channel DirectSound
  inputs and selected ASIO inputs as separate CHOP channels, with explicit
  device, rate and buffer length
  ([Audio Device In CHOP](https://docs.derivative.ca/Audio_Device_In_CHOP)).

## Current evidence

| Area | TouchDesigner reference | Syndocal today | Assessment |
|---|---|---|---|
| Live presentation | [Perform Mode](https://docs.derivative.ca/Perform_Mode) renders one selected Window COMP without the network editor; Windows full-screen exclusive can avoid compositor stutter | A dedicated VJ Desk, automatic maximize, exact F11 1920x1080 and separate Program outputs; no exclusive-swapchain proof | Operational surface is now comparable in shape, but performance parity is unproven |
| Audio capture | [Audio Device In CHOP](https://docs.derivative.ca/Audio_Device_In_CHOP) uses DirectSound/WDM or ASIO on Windows and exposes device, actual input channels, rate and buffer length | On Windows, CPAL/WASAPI shared exposes a generation-scoped device catalogue, explicit rate/requested-buffer and Average All/Single/Stereo Pair downmix. A separately built non-default ASIO bridge adds explicit driver/rate/channels/native format/fixed buffer selection, post-open buffer and XRUN reporting, and terminal fail-closed events without first-driver or WASAPI fallback. TOPPING 48 kHz / 2-channel / i32 / 128-frame short smoke, 100 Start/Stop/Free cycles and the production 1920x1080 F11 operator path passed; Realtek ASIO failed honestly as unavailable | The explicit ASIO open/operator-path gap is closed on one device. Syndocal currently collapses the selected mix to mono analysis; TouchDesigner preserves a true multi-channel CHOP graph and still leads in backend and routing breadth. Distribution, second-driver, endurance, latency and matched runtime parity remain open |
| Audio operator surface | Parameters and CHOP viewers can be arranged in a custom TouchDesigner network/UI | The VJ Desk keeps a compact input/telemetry rail and adds a 54 px REACTIVE strip that does not cover Preview/Program. One action opens a dense Audio Reactive Rack with source, feature, response, target and live IN/OUT state in one view. Stateful viewport gates cover the live rail at five English/Japanese sizes and the rack at 1920x1080/1366x768 | Syndocal exposes a purpose-built one-view prepared-show path; TouchDesigner remains more freely arrangeable, and matched task-time evidence is still open |
| Spectrum/features | [Audio Spectrum CHOP](https://docs.derivative.ca/Audio_Spectrum_CHOP) offers FFT sizes 64–16384, manually selectable output length and magnitude/phase channels; [Analyze CHOP](https://docs.derivative.ca/Analyze_CHOP) provides RMS, extrema and peak analysis; [Palette:audioAnalysis](https://docs.derivative.ca/Palette%3AaudioAnalysis) exposes low/mid/high, kick/snare/rhythm, centroid and slow/fast density | The 48 kHz path uses a construction-time allocated 2048-point / 16-band logarithmic FFT, RMS/Peak, spectral flux, adaptive onset/strength, 60–200 BPM/confidence/phase, centroid, 80/800 ms density, kick/snare events and strengths, plus compatible B/M/H derivation | Syndocal covers the named prepared-show descriptors and adds a fixed confidence-scored director input; TouchDesigner leads in FFT/output configurability, magnitude/phase access and arbitrary per-channel composition |
| Audio mapping | CHOP channels can feed a general-purpose procedural operator network | Saved Audio Rack mappings expose 16 bands and every named descriptor, gain/bias/gate, attack/release/hold, four curves and invert, then drive supported lighting attributes or video parameters. Values are cached once per engine tick; stale/unverified input restores authored state instead of baking modulation into the project | Syndocal provides a purpose-built, product-level fail-closed path for the fixed show task; TouchDesigner remains stronger for unconstrained analysis and graph authoring, while matched speed evidence is still open |
| Beat / Auto VJ | [Beat CHOP](https://docs.derivative.ca/Beat_CHOP) outputs ramps, pulses, beat/bar/count and BPM from configured or tapped tempo | Auto VJ explicitly selects CLOCK or LIVE INPUT. Live mode consumes each real onset sequence once; a deterministic seed/show revision/candidate order produces logged Takes at the configured pulse interval. Manual Take latches Hold, blackout wins, input loss holds and project load is Off. Program Audio uses the same backend-owned, generation-tokened handoff path for automatic and manual Takes; slow device/decode work does not block Take | Software control, audio handoff and safety paths are implemented; dataset accuracy, long-run and matched latency evidence remain open |
| Runtime profiling | [Performance Monitor](https://docs.derivative.ca/Performance_Monitor_Dialog) records cook/UI/display events and triggered slow frames; [Perform CHOP](https://docs.derivative.ca/Perform_CHOP) exposes FPS, cook state/time slice and memory; [Probe](https://docs.derivative.ca/Palette%3Aprobe) visualizes CPU/GPU time and memory | Engine, audio callback/queue, DMX, decoder and selected renderer telemetry exist, but there is no single production frame graph covering capture → analysis → engine → GPU → display with comparable history | TouchDesigner has the broader interactive profiling toolkit. Syndocal needs unified per-stage frame telemetry before claiming easier diagnosis or lower runtime cost |
| Video interchange | [NDI In](https://docs.derivative.ca/NDI_In_TOP) receives audio/metadata and can hardware-decode NDI\|HX; [NDI Out](https://docs.derivative.ca/NDI_Out_TOP) sends audio/metadata, failover identity and HDR-aware color-space metadata (software output is native NDI, not HX). [Syphon/Spout Out](https://docs.derivative.ca/Syphon_Spout_Out_TOP) supports Spout through 32-bit float RGBA and Syphon 8-bit RGBA | NDI 6 discovery/RGBA/30000/1001 receive, 30 fps send, scaling and connection retention passed against NDI tools; Spout RGBA input/output and reconnect passed against TouchDesigner. NDI audio/metadata/failover/HDR/HX and float Spout/Syphon breadth are not accepted | Syndocal's NDI 6 and Spout interoperability are real implementation evidence, not breadth parity. TouchDesigner leads on media metadata/audio/failover/HDR/HX and high-precision/local-platform interchange |
| Control and show I/O | [OSC Out CHOP](https://docs.derivative.ca/OSC_Out_CHOP) sends timestamped UDP OSC channels/events and supports bundles through OSC Out DAT; [MIDI In CHOP](https://docs.derivative.ca/MIDI_In_CHOP) can match SysEx while [MIDI Out CHOP](https://docs.derivative.ca/MIDI_Out_CHOP) sends 7/14-bit controllers; [DMX Out CHOP](https://docs.derivative.ca/DMX_Out_CHOP) includes Art-Net, sACN, KiNET and serial/USB devices; [Serial DAT](https://docs.derivative.ca/Serial_DAT) receives and sends binary or delimited RS-232 data | OSC/MIDI mapping, Art-Net/sACN/serial DMX, dedicated RDM console/core and unified cue/video control are implemented. General serial input, KiNET, MIDI SysEx and full 14-bit controller acceptance are absent or unproven | TouchDesigner leads in generic protocol breadth. Syndocal's integrated RDM and show-safe domain model are specialization advantages only after matched operator/error-recovery trials |
| Safety | General networks can be built for fallback behavior | Stream fault or 250 ms without input requests a shared lighting/video clear, retains its retry owner while pending, and the Engine independently expires an unrefreshed live value after 250 ms | Syndocal has a product-level fail-closed contract; applied-clear acknowledgement remains open |
| Authoring | General operator network and custom component model | Fixed Source → Transform → Output graph and one video effect slot per layer | TouchDesigner leads decisively |
| Unified show control | Can be constructed from operators/protocols | Lighting Cue, video Take, clock, blackout, Timeline, MIDI/OSC/DMX and audio source share one show model | Syndocal has a simpler dedicated workflow |

Syndocal's product-level safety contract, compiled lighting/video cross-domain
mapping, deterministic Auto VJ, authoritative Manual Take/Hold/Blackout, RDM
workflow and verified NDI 6 interoperability are credible advantage candidates
for prepared shows. None is a superiority result until the matched tests below
measure task success, recovery and runtime cost against the frozen Official
Build project.

## Required implementation order

1. Keep the implemented preallocated capture pool, callback min/max, queue
   high-water and timestamp-derived capture-to-worker estimate, then add
   callback duration plus capture-to-analysis/publish percentiles.
2. Keep the implemented explicit WASAPI shared device, channel mix, sample rate
   and requested buffer configuration, then add same-device reconnect. Never
   silently switch to a different device during a show.
3. Keep the separately buildable ASIO bridge, completed TOPPING 100-cycle gate
   and explicit-driver fail-closed contract, then complete second-vendor, one-hour and fault-injection
   acceptance. Do not enable it in distributed binaries until the [Steinberg
   ASIO SDK/license path](https://www.steinberg.net/developers/asiosdk-open/) is
   explicitly selected and the separate artifact obligations are recorded.
4. Keep the implemented RMS/peak, 16 bands, spectral flux, adaptive onset,
   centroid/density, kick/snare, attack/release and confidence-scored BPM/phase,
   then add frozen-dataset accuracy evidence. The tracker does not silently
   retime the master show clock.
5. Keep the implemented compiled Audio Reactive Rack and authored/rendered
   snapshot separation, then add reusable mapping templates and measured
   capture-to-engine/pixel percentiles. Continuous modulation and event pulses
   remain separate runtime concepts. Measure 44 Hz tick jitter while large
   projects are saved, checkpointed and written by warm-standby sync before
   treating on-demand persistence as real-time-safe.
6. Keep the implemented deterministic Auto VJ core and live-onset source, then
   add style/energy presets, descriptor-driven effect modulation and long-run
   real-show evidence. The same accepted onset sequence, seed and show revision
   must generate the same action log.
7. Move multi-effect video evaluation to a compiled GPU graph and keep ordinary
   frames GPU-resident through output. Add per-stage decode/render/output
   telemetry before making a performance claim.

## Quantitative acceptance

### Audio runtime

- Current native smoke evidence is one maximized current-source VJ Desk run in a
  1920x1032 Windows work area. It reached Ready without a command mismatch and
  started/stopped the system-default WASAPI shared input:
  48 kHz mono, callback current/min/max 480/480/480 frames,
  sampled capture-to-worker current 0.1 ms/maximum 10.1 ms, overrun 0/0 and queue
  current/high-water/capacity 0/1/4; Start and Stop passed. It does not satisfy
  the one-hour, latency-percentile or cross-hardware gates below.
- Current ASIO evidence begins with one short bridge/app-loader hardware smoke with
  `TOPPING Pro USB Audio Device`: requested and applied 48 kHz / 2 channels /
  i32 / 128 frames, 7 callbacks, 896 frames, maximum capture delay 4166.7 us,
  XRUN 0, nonfinite 0, terminal event 0, and successful Stop. An explicit
  `Realtek ASIO` attempt returned hardware unavailable and did not fall back to
  another driver or WASAPI. The exact TOPPING configuration then passed 100
  Start/Stop/Free cycles in 27.6767468 seconds with actual buffer 128 every
  cycle, 200 callbacks and zero warning, terminal, XRUN, nonfinite, frame
  mismatch or fallback events. Finally, the current-source native VJ Desk
  explicitly selected the same driver/rate/buffer, reached Active with OVR 0/0
  and XRUN 0 in F11 1920x1080, stopped to Ready and returned with Esc. This does
  not pass the second-driver, one-hour, capture-to-engine/pixel, physical-latency
  or matched TouchDesigner gates.
- Normal audio data callback heap allocation and mutex acquisition: **0**
  (implemented structurally; the one-hour and instrumented-allocation gates
  remain open).
- 48 kHz / 128-frame ASIO and 48 kHz / 256-frame WASAPI, one hour: ring
  overrun **0**.
- Callback duration: p99 below 20% of buffer duration, maximum below 50%.
- Feature extraction: p99 below 2 ms and below 5% of one CPU core.
- Compiled Audio Rack routing, 64 mappings x 200 lighting targets: the current
  1000-sample release microbenchmark is p99 196 us / max 502 us and passes its
  p99 1 ms / max 2 ms gate. This does not measure capture or pixels.
- Capture timestamp → engine feature application: ASIO p95 ≤ 40 ms, WASAPI
  p95 ≤ 80 ms.
- Input loss → lighting/video modulation zero: ≤ 250 ms.
- Same-device reconnect: ≤ 5 s; different-device fallback requires operator
  confirmation.

### Beat and Auto VJ

- Onset F1 score ≥ 0.90 on the frozen validation set.
- BPM 60–200: median absolute error ≤ 0.5 BPM, p95 ≤ 2 BPM, initial lock ≤ 8 s.
- Stable beat phase error: p95 ≤ 25 ms.
- Impulse → feature p95 ≤ 30 ms; impulse → output pixel p95 ≤ 50 ms and
  p99 ≤ 66.7 ms at 60 Hz.
- Same audio + seed + show revision: identical Auto VJ event log.
- Two-hour Auto VJ run: crash, invalid clip selection and unintended black
  frame **0**.
- Manual Take/Hold/Blackout remains authoritative; Take/Hold response p95
  ≤ 33 ms and blackout p99 ≤ one output frame.

### Performance comparison

Freeze a TouchDesigner **2025.32820** `.toe` and the matching Syndocal `.sdc`.
On the same machine/GPU driver, use the same `TOPPING Pro USB Audio Device`,
48 kHz / 128 frames, 2048-point FFT, source material, 1920x1080/60 output and
equivalent effect chain. Warm both applications, then execute **five measured
runs each** in alternating order.

For every run, report capture-to-feature, capture-to-engine and physical
audio-to-visible-pixel p50/p95/p99/max; frame-time p50/p95/p99/max; dropped and
late frames; process CPU; GPU utilization/time; and RAM/VRAM. Record startup,
input-loss-to-safe-state and Manual Take/Hold/Blackout task results separately.
Declare the primary metric or metrics before running the trial; the default
primaries are capture-to-feature p95 and audio-to-visible-pixel p95.

"Equal or better" requires no crash, safety/recovery failure, dropped-frame
regression or other critical degradation, with no material regression in the
declared guard metrics. "Better" additionally requires at least a **10%**
improvement in one predeclared primary metric. A faster isolated routing
microbenchmark or the seven-callback ASIO smoke cannot satisfy this gate.

## Claim boundary

Until the A/B gates above pass, release language must say:

- Syndocal provides a dedicated unified lighting/VJ desk with safe live
  16-band microphone analysis and an explicitly armed deterministic Auto VJ
  that can follow the shared clock or counted live-input onsets.
- It does not yet provide TouchDesigner-class general visual programming,
  distribution-ready and cross-hardware ASIO/multichannel breadth, matched
  dataset accuracy, GPU graph breadth or proven end-to-end performance parity.
