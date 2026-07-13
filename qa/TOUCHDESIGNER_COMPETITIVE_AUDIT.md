# TouchDesigner / Auto VJ Competitive Audit

Updated: 2026-07-14

## Verdict

Syndocal is not yet equal to TouchDesigner as a general real-time visual
programming environment. It can become stronger for a narrower but valuable
job: running a prepared lighting-and-video show quickly, safely and
deterministically without constructing an operator network at the venue.

The comparison must therefore keep two claims separate:

1. **General authoring parity:** arbitrary graph construction, GPU-resident
   texture processing, device/protocol breadth and reusable tools. This remains
   open and is a large product boundary.
2. **Prepared show / Auto VJ superiority:** faster setup, clearer live state,
   lower operator workload, synchronized lighting/video reaction and
   deterministic recovery. This is an achievable Syndocal-specific target.

## Current evidence

| Area | TouchDesigner reference | Syndocal today | Assessment |
|---|---|---|---|
| Live presentation | [Perform Mode](https://docs.derivative.ca/Perform_Mode) renders one selected Window COMP without the network editor; Windows full-screen exclusive can avoid compositor stutter | A dedicated VJ Desk, automatic maximize, exact F11 1920x1080 and separate Program outputs; no exclusive-swapchain proof | Operational surface is now comparable in shape, but performance parity is unproven |
| Audio capture | [Audio Device In CHOP](https://docs.derivative.ca/Audio_Device_In_CHOP) exposes default/native/ASIO drivers, device, channel, rate and buffer selection | On Windows, CPAL/WASAPI shared exposes a generation-scoped device catalogue, per-device capabilities, explicit rate/requested-buffer and Average All/Single/Stereo Pair downmix. The selected rate resolves one channel/sample-format/buffer-capability tuple which Start revalidates; unsupported configurations and stale IDs fail instead of silently falling back. The bounded allocation-free callback accepts all 11 CPAL PCM formats and reports callback/drop/queue/capture-to-worker telemetry | The WASAPI shared configuration gap is materially reduced, but runtime parity is unproven; TouchDesigner still leads in ASIO, general multichannel analysis and backend breadth |
| Audio operator surface | Parameters and CHOP viewers can be arranged in a custom TouchDesigner network/UI | One 64 px full-window rail shows device/config or four live telemetry badges plus B/M/H. Lightweight meters update at about 30 Hz, full telemetry at about 1 Hz, and Refresh is single-flight. 1280x720 and 1920x1080 stopped/live containment gates pass | Syndocal now has the faster purpose-built show surface; TouchDesigner remains more configurable |
| Spectrum/features | [Audio Spectrum CHOP](https://docs.derivative.ca/Audio_Spectrum_CHOP) exposes configurable FFT sizes and magnitude/phase; [Analyze CHOP](https://docs.derivative.ca/Analyze_CHOP) provides RMS, extrema and peak analysis | 1024-sample Hann FFT, Bass/Mid/High, silence gate and smoothing | TouchDesigner leads in general analysis breadth |
| Beat | [Beat CHOP](https://docs.derivative.ca/Beat_CHOP) outputs ramps, pulses, beat/bar/count and BPM from configured or tapped tempo | Shared BPM/clock exists, but live microphone onset/BPM tracking is not implemented | Neither reference proves turnkey automatic beat detection; this is the clearest place for Syndocal to lead |
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
4. Expand analysis to RMS/peak, 8/16 bands, spectral flux, centroid, onset,
   adaptive floor and per-feature attack/release/gate/curve.
5. Add confidence-scored live BPM and beat/bar phase. A low-confidence tracker
   must not silently retime the master show clock.
6. Build an Audio Reactive Rack with explicit Source, Target, Range, Curve,
   Attack, Release, Gate, Invert and Hold. Continuous modulation and event
   triggers must be separate concepts.
7. Add deterministic Auto VJ presets. Clip changes occur on configured beats or
   bars; the same audio, seed and show file must generate the same action log.
8. Move multi-effect video evaluation to a compiled GPU graph and keep ordinary
   frames GPU-resident through output. Add per-stage decode/render/output
   telemetry before making a performance claim.

## Quantitative acceptance

### Audio runtime

- Current native smoke evidence is one Windows system-default WASAPI shared run:
  48 kHz mono, callback current/min/max 480/480/480 frames,
  sampled capture-to-worker current 10.0 ms/maximum 10.2 ms, overrun 0/0 and queue
  current/high-water/capacity 0/1/4; Start and Stop passed. It does not satisfy
  the one-hour, ASIO, latency-percentile or cross-hardware gates below.
- Normal audio data callback heap allocation and mutex acquisition: **0**
  (implemented structurally; the one-hour and instrumented-allocation gates
  remain open).
- 48 kHz / 128-frame ASIO and 48 kHz / 256-frame WASAPI, one hour: ring
  overrun **0**.
- Callback duration: p99 below 20% of buffer duration, maximum below 50%.
- Feature extraction: p99 below 2 ms and below 5% of one CPU core.
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

Until the implementation and A/B gates above pass, release language must say:

- Syndocal provides a dedicated unified lighting/VJ desk with safe live
  three-band microphone modulation.
- It does not yet provide TouchDesigner-class general visual programming,
  ASIO/multichannel analysis, GPU graph breadth or automatic VJ.
