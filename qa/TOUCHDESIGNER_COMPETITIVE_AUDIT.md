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

## Current evidence

| Area | TouchDesigner reference | Syndocal today | Assessment |
|---|---|---|---|
| Live presentation | [Perform Mode](https://docs.derivative.ca/Perform_Mode) renders one selected Window COMP without the network editor; Windows full-screen exclusive can avoid compositor stutter | A dedicated VJ Desk, automatic maximize, exact F11 1920x1080 and separate Program outputs; no exclusive-swapchain proof | Operational surface is now comparable in shape, but performance parity is unproven |
| Audio capture | [Audio Device In CHOP](https://docs.derivative.ca/Audio_Device_In_CHOP) exposes default/native/ASIO drivers, device, channel, rate and buffer selection | On Windows, CPAL/WASAPI shared exposes a generation-scoped device catalogue, per-device capabilities, explicit rate/requested-buffer and Average All/Single/Stereo Pair downmix. The selected rate resolves one channel/sample-format/buffer-capability tuple which Start revalidates; unsupported configurations and stale IDs fail instead of silently falling back. The bounded allocation-free callback accepts all 11 CPAL PCM formats and reports callback/drop/queue/capture-to-worker telemetry | The WASAPI shared configuration gap is materially reduced, but runtime parity is unproven; TouchDesigner still leads in ASIO, general multichannel analysis and backend breadth |
| Audio operator surface | Parameters and CHOP viewers can be arranged in a custom TouchDesigner network/UI | The VJ Desk keeps a compact input/telemetry rail and adds a 54 px REACTIVE strip that does not cover Preview/Program. One action opens a dense Audio Reactive Rack with source, feature, response, target and live IN/OUT state in one view. Stateful viewport gates cover the live rail at five English/Japanese sizes and the rack at 1920x1080/1366x768 | Syndocal exposes a purpose-built one-view prepared-show path; TouchDesigner remains more freely arrangeable, and matched task-time evidence is still open |
| Spectrum/features | [Audio Spectrum CHOP](https://docs.derivative.ca/Audio_Spectrum_CHOP) exposes configurable FFT sizes and magnitude/phase; [Analyze CHOP](https://docs.derivative.ca/Analyze_CHOP) provides RMS, extrema and peak analysis; the [audioAnalysis palette component](https://docs.derivative.ca/Palette%3AaudioAnalysis) exposes low/mid/high, kick/snare/rhythm, spectral centroid and slow/fast density | Construction-time allocated 16-band logarithmic FFT, RMS/Peak, spectral flux, adaptive onset/strength, 60–200 BPM/confidence/phase, centroid, 80/800 ms density, kick/snare events and strengths, plus compatible B/M/H derivation | Syndocal documents the named audioAnalysis palette descriptors plus additional fixed-rack descriptors for prepared shows; TouchDesigner still leads in configurable FFT size, magnitude/phase channels and arbitrary CHOP composition |
| Audio mapping | CHOP channels can feed a general-purpose procedural operator network | Saved Audio Rack mappings expose 16 bands and every named descriptor, gain/bias/gate, attack/release/hold, four curves and invert, then drive supported lighting attributes or video parameters. Values are cached once per engine tick; stale/unverified input restores authored state instead of baking modulation into the project | Syndocal provides a purpose-built, product-level fail-closed path for the fixed show task; TouchDesigner remains stronger for unconstrained analysis and graph authoring, while matched speed evidence is still open |
| Beat / Auto VJ | [Beat CHOP](https://docs.derivative.ca/Beat_CHOP) outputs ramps, pulses, beat/bar/count and BPM from configured or tapped tempo | Auto VJ explicitly selects CLOCK or LIVE INPUT. Live mode consumes each real onset sequence once; a deterministic seed/show revision/candidate order produces logged Takes at the configured pulse interval. Manual Take latches Hold, blackout wins, input loss holds and project load is Off. Program Audio uses the same backend-owned, generation-tokened handoff path for automatic and manual Takes; slow device/decode work does not block Take | Software control, audio handoff and safety paths are implemented; dataset accuracy, long-run and matched latency evidence remain open |
| Safety | General networks can be built for fallback behavior | Stream fault or 250 ms without input requests a shared lighting/video clear, retains its retry owner while pending, and the Engine independently expires an unrefreshed live value after 250 ms | Syndocal has a product-level fail-closed contract; applied-clear acknowledgement remains open |
| Authoring | General operator network and custom component model | Fixed Source → Transform → Output graph and one video effect slot per layer | TouchDesigner leads decisively |
| Unified show control | Can be constructed from operators/protocols | Lighting Cue, video Take, clock, blackout, Timeline, MIDI/OSC/DMX and audio source share one show model | Syndocal has a simpler dedicated workflow |

## Required implementation order

1. Keep the implemented preallocated capture pool, callback min/max, queue
   high-water and timestamp-derived capture-to-worker estimate, then add
   callback duration plus capture-to-analysis/publish percentiles.
2. Keep the implemented explicit WASAPI shared device, channel mix, sample rate
   and requested buffer configuration, then add same-device reconnect. Never
   silently switch to a different device during a show.
3. Add a separately buildable ASIO feature and real-device acceptance. Do not
   enable it in distributed binaries until the [Steinberg ASIO SDK/license
   path](https://www.steinberg.net/developers/asiosdk-open/) is explicitly
   selected.
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
  the one-hour, ASIO, latency-percentile or cross-hardware gates below.
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

Use the same machine, GPU driver, audio interface, 48 kHz sample rate, buffer,
FFT size, media, 1920x1080/60 output and equivalent effect chain. Freeze the
TouchDesigner `.toe` and Syndocal `.sdc`, warm both runs, execute five trials,
and report p50/p95/p99 plus dropped frames, CPU, GPU, VRAM and capture-to-pixel
latency. Syndocal may claim superiority only when no critical metric regresses
and at least one declared primary metric improves by 10% or more.

## Claim boundary

Until the A/B gates above pass, release language must say:

- Syndocal provides a dedicated unified lighting/VJ desk with safe live
  16-band microphone analysis and an explicitly armed deterministic Auto VJ
  that can follow the shared clock or counted live-input onsets.
- It does not yet provide TouchDesigner-class general visual programming,
  ASIO/multichannel breadth, matched dataset accuracy, GPU graph breadth or
  proven end-to-end performance parity.
