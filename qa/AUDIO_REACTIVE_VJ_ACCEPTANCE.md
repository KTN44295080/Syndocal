# Audio-Reactive and Auto VJ Acceptance

Updated: 2026-07-14

## Competitive baseline

TouchDesigner provides device input through DirectSound/CoreAudio or ASIO, explicit device/rate/buffer/channel controls, time-sliced channel data, FFT sizes from 64 through 16384, arbitrary channel analysis such as RMS and peaks, and a palette audio-analysis component exposing low/mid/high, kick, snare, rhythm, spectral centroid, and slow/fast spectral density.

Primary references:

- Audio Device In CHOP: <https://docs.derivative.ca/Audio_Device_In_CHOP>
- Audio Spectrum CHOP: <https://docs.derivative.ca/Audio_Spectrum_CHOP>
- Analyze CHOP: <https://docs.derivative.ca/Analyze_CHOP>
- Palette audioAnalysis: <https://docs.derivative.ca/Palette%3AaudioAnalysis>

Syndocal does not claim general TouchDesigner parity from a fixed VJ rack. The target here is a purpose-built, fail-closed show-operation path for audio-reactive VJ while keeping the procedural Node Graph available for custom mappings.

## Feature-frame contract

Every live analysis frame must expose:

- 16 stable logarithmic frequency bands covering the useful music range up to Nyquist;
- linear RMS and peak;
- positive spectral flux and an onset pulse with a refractory interval;
- spectral centroid, independent slow/fast spectral density, and kick/snare event plus strength values;
- estimated BPM, confidence, and beat phase when the evidence is sufficient;
- a monotonic sequence/timestamp so an onset cannot trigger twice;
- the existing bass/mid/high compatibility values derived from the same window.

The capture callback stays bounded and allocation-free. FFT, history, onset, and BPM work remain on the existing analysis worker with construction-time allocation only. Silence, NaN/Inf, stale input, and engine clear must produce a zero feature frame.

## Audio Reactive Rack

- All 16 bands are visible at 1920x1032 and 1920x1080 without covering Preview or Program.
- RMS, peak, onset, BPM/confidence, input health, and active backend are readable without opening diagnostics.
- A mapping declares source, attack, release, gain, bias, curve, target parameter, low/high range, and blend mode.
- Mappings can target video opacity/speed/transform/color/FX and lighting attributes supported by Node Graph outputs.
- Mapping creation is explicit; saved mappings can be enabled/disabled, removed, and saved/loaded as presets. Reactive runtime values do not write into the operator's base layer/fixture state. Editing an existing mapping in place is not yet claimed.
- Missing input, stale input, or manual bypass returns modulation to neutral within 250 ms.

## Deterministic Auto VJ director

- The director is explicitly Armed; project load never starts it or enables an output.
- A seed, show revision, eligible ordered clips, and action sequence fully determine clip choice and action log.
- Actions occur once at the configured beat/bar boundary. A repeated engine tick cannot duplicate a Take.
- Avoid-repeat is guaranteed whenever at least two eligible clips exist.
- Hold produces no actions. Manual Take wins in the same tick and latches Hold/manual priority.
- Lighting/video blackout is never cleared by Auto VJ.
- Missing or invalid candidates fail to Hold/Fault without changing Program.
- Disarming restores operator control without rewriting the selected clip's authored state.
- Every action records sequence, boundary, seed/revision context, target clip, transition, and reason.

## Performance and end-to-end gates

- Analysis-worker p99 stays below 20% of a 33.3 ms analysis period and max below 50% during a one-hour run.
- Capture callback overrun/drop remains zero under the matched hardware configuration.
- Capture-to-engine p95 is at most 40 ms.
- Physical audio marker to visible pixel p95 is measured at 48 kHz with fixed 128/256 frame buffers.
- Run the same content and interface for five trials in Syndocal and TouchDesigner; report median and worst trial rather than selecting the fastest run.
- 1280x720, 1366x768, 1920x1032, 1920x1080, and 2048x1152 pass English/Japanese stateful viewport gates for stopped, live, onset, low-confidence BPM, Running, Hold, and Fault.

## Claim boundary

Passing unit or synthetic-tone tests proves algorithm behavior, not production parity. TouchDesigner-equivalent or better audio-reactive VJ can be claimed only after real hardware, physical latency, one-hour stability, operator workflow, and matched five-trial evidence are attached to this file.

## Current implementation status (2026-07-14)

- Implemented: fixed-allocation 16-band feature extraction, RMS/peak, spectral flux, adaptive onset, BPM/confidence, beat phase, and legacy B/M/H derivation.
- Implemented: spectral centroid, 80 ms/800 ms fast/slow spectral density, kick/snare strength, and independently refractory kick/snare pulses. The normal analysis path reuses its construction-time FFT/window/history storage.
- Implemented: a compiled Audio Reactive Rack over the saved Node Graph model. Live Input exposes 16 bands, RMS, peak, flux, onset/strength, BPM/confidence/phase, centroid, both densities, kick/snare events and strengths; response controls include gain, bias, gate, attack, release, hold, four curves and invert. Lighting and video outputs read a cached value once per engine tick instead of recursively walking nodes per target. Reactive video Speed and BPM-sync parameters drive the real playhead while leaving authored transport values unchanged; source loss resumes the authored speed immediately.
- Implemented: stale, unverified, cleared, or older-than-250-ms Live Input immediately returns the rack to safe zero and restores authored fixture/video state. Onset, kick and snare pulses are latched for every graph exactly once per tick. Project save, checkpoint/history and Cue capture use an on-demand authored-video snapshot, so live modulation is not baked into the project or applied twice after reload.
- Implemented: graph create/update, enable and remove use a publication acknowledgement and roll back the complete graph/cue state if publication fails.
- Implemented: deterministic Auto VJ with Clock or Live Audio onset source, exact-once feature sequence consumption, quantized change interval, avoid-repeat, action evidence, manual-Take Hold latch, blackout preservation, published-snapshot acknowledgement, rollback, and safe project-load Off state.
- Implemented: the compact live rail exposes all 16 bands and rhythm telemetry without adding more than the existing three accessible compatibility meters.
- Implemented: Program Audio handoff is owned by a generation-tokened backend coordinator. Auto and manual Takes share one exactly-once path; slow device open/decode/sink work does not block Take, while Off, gain/device reconfiguration, direct Launch/Deck monitoring, and project load are reconciled explicitly.
- Automated evidence: audio 22/22, protocol 19/19, engine 284/284 with one manual benchmark ignored in the normal suite, default current-source Tauri 269 passed / 9 ignored, and ASIO no-default-features Tauri 268 passed / 6 ignored. The isolated ASIO bridge passes its SDK-free 5-test suite and SDK-enabled 4-test suite; the app's ignored explicit-driver catalogue/capability integration test was also run manually against the built bridge. Frontend build, project-storage and 2298/2298 localization checks pass. Audio Rack containment passes at 1920x1080 and 1366x768 with zero rail/rack overflow and no decorative canvas; its monitor selects a saved Audio graph and shows only the matching engine node's authoritative IN/OUT/safety telemetry. English/Japanese Auto VJ and live-audio gates pass at five sizes.
- Release benchmark evidence: 64 live mappings routed to 200 lighting targets each completed 1000 release-mode samples at p99 196 us and maximum 502 us, below the rack-only p99 1 ms / maximum 2 ms gate. This is a CPU routing microbenchmark, not capture-to-pixel or whole-renderer evidence.
- Native smoke evidence: the current-source VJ Desk was inspected maximized in the 1920x1032 Windows work area, reached Ready without a command mismatch, and started/stopped the system-default 48 kHz WASAPI shared microphone. The sampled callback was 480/480/480 frames, capture-to-worker current/maximum 0.1/10.1 ms, overrun 0/0, and queue 0/1/4. This is a single stopped/silent-microphone smoke run, not an accuracy or endurance result.
- ASIO implementation and hardware smoke: the non-default Windows feature dynamically loads the independently built ABI v1 bridge and exposes explicit backend, driver, rate, channels, native format, fixed buffer, applied buffer and backend XRUN state in the existing live-audio UI. It provides no system-default ASIO selection and does not fall back to another ASIO driver or WASAPI. With `TOPPING Pro USB Audio Device` explicitly selected, 48 kHz / 2 channels / i32 / requested and applied 128 frames started, delivered 7 callbacks and 896 frames with maximum capture delay 4166.7 us, XRUN 0, nonfinite 0 and terminal event 0, then stopped successfully. `Realtek ASIO` returned hardware input/output unavailable and remained failed without fallback. This bridge/app-loader run is a short smoke on one working driver, not endurance, latency-percentile, or cross-hardware acceptance.
- ASIO repeated-open evidence: the bridge ignored hardware test ran `--locked --offline` against explicit ID `asio:TOPPING Pro USB Audio Device` at 48 kHz / 2 channels / i32 / 128 frames and passed 100 Start/Stop/Free cycles in 27.6767468 seconds. All 100 applied buffers were 128 frames; there were 200 callbacks with at least two per cycle, Stop 100, Free 100, and zero warnings, terminal events, XRUN events/API count, nonfinite samples, frame mismatches or fallback. This completes the repeated-cycle gate, but is not a one-hour soak, cross-vendor, hot-plug, physical-latency or native full-UI result.
- Native UI evidence: a separately identified current-source QA build was inspected maximized at 1920x1032 without closing the existing Syndocal or Daslight windows. The 54 px REACTIVE strip preserved both Preview and Program monitors, Create Mapping opened the production Effects desk, Live Input enabled the rich feature selector, `Snare strength` could be selected, and stopped input remained visibly `SAFE ZERO`. After the P1 telemetry correction, the QA binary was rebuilt and rechecked at the same maximized size: an empty rack showed `0/0 READY`, and its editor showed `No saved mapping` plus `SAVE TO MONITOR` rather than presenting draft arithmetic as engine output. No clipping or horizontal overlap was observed.
- Native ASIO UI evidence: a current-source non-default ASIO QA build was operated at the primary production size, not a small acceptance window. The maximized window capture was 1913x1080 including Windows chrome; F11 produced an exact 1920x1080 capture at origin 0,0 without title bar or taskbar. From the VJ Desk, the operator selected ASIO, `TOPPING Pro USB Audio Device`, 48.0 kHz, 128 frames and Average All → mono, then Start reached `ACTIVE` with OVR 0/0 and XRUN 0. Stop returned the rail to `READY`, and Esc restored the maximized window. This closes the native full-UI path only; it is not a one-hour, physical-latency, callback-percentile or TouchDesigner parity result.
- Pending: ASIO distribution-license/artifact approval, a second working ASIO vendor driver, same-device hot-plug recovery, general post-open applied-config reporting beyond the ASIO buffer result, capture-to-analysis/engine/pixel latency percentiles, frozen-dataset onset/BPM/kick/snare accuracy, physical latency, matched one-hour ASIO/WASAPI hardware soaks, large-show save/checkpoint/warm-standby tick-jitter evidence, and matched five-trial TouchDesigner runs.

This status is an implementation inventory, not an acceptance pass. Check off performance and parity gates only when their recorded evidence is added below.
