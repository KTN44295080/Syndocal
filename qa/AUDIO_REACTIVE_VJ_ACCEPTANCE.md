# Audio-Reactive and Auto VJ Acceptance

Updated: 2026-07-14

## Competitive baseline

TouchDesigner provides device input through DirectSound/CoreAudio or ASIO, explicit device/rate/buffer/channel controls, time-sliced channel data, FFT sizes from 64 through 16384, arbitrary channel analysis such as RMS and peaks, and a palette audio-analysis component exposing low/mid/high, kick, snare, rhythm, spectral centroid, and slow/fast spectral density.

Primary references:

- Audio Device In CHOP: <https://docs.derivative.ca/Audio_Device_In_CHOP>
- Audio Spectrum CHOP: <https://docs.derivative.ca/Audio_Spectrum_CHOP>
- Analyze CHOP: <https://docs.derivative.ca/Analyze_CHOP>
- Palette audioAnalysis: <https://docs.derivative.ca/Palette%3AaudioAnalysis>

Syndocal does not claim general TouchDesigner parity from a fixed VJ rack. The target here is a faster, safer show-operation path for audio-reactive VJ while keeping the procedural Node Graph available for custom mappings.

## Feature-frame contract

Every live analysis frame must expose:

- 16 stable logarithmic frequency bands covering the useful music range up to Nyquist;
- linear RMS and peak;
- positive spectral flux and an onset pulse with a refractory interval;
- estimated BPM, confidence, and beat phase when the evidence is sufficient;
- a monotonic sequence/timestamp so an onset cannot trigger twice;
- the existing bass/mid/high compatibility values derived from the same window.

The capture callback stays bounded and allocation-free. FFT, history, onset, and BPM work remain on the existing analysis worker with construction-time allocation only. Silence, NaN/Inf, stale input, and engine clear must produce a zero feature frame.

## Audio Reactive Rack

- All 16 bands are visible at 1920x1032 and 1920x1080 without covering Preview or Program.
- RMS, peak, onset, BPM/confidence, input health, and active backend are readable without opening diagnostics.
- A mapping declares source, attack, release, gain, bias, curve, target parameter, low/high range, and blend mode.
- Mappings can target video opacity/speed/transform/color/FX and lighting attributes supported by Node Graph outputs.
- Live edits are reversible and do not write reactive values into the operator's base layer/fixture state.
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
- Implemented: deterministic Auto VJ with Clock or Live Audio onset source, exact-once feature sequence consumption, quantized change interval, avoid-repeat, action evidence, manual-Take Hold latch, blackout preservation, published-snapshot acknowledgement, rollback, and safe project-load Off state.
- Implemented: the compact live rail exposes all 16 bands and rhythm telemetry without adding more than the existing three accessible compatibility meters.
- Implemented: Program Audio handoff is owned by a generation-tokened backend coordinator. Auto and manual Takes share one exactly-once path; slow device open/decode/sink work does not block Take, while Off, gain/device reconfiguration, direct Launch/Deck monitoring, and project load are reconciled explicitly.
- Automated evidence: audio 19/19, protocol 19/19, engine 275/275, Tauri live-input 30/30, Program Audio 10/10, frontend build/storage/localization, and English/Japanese Auto VJ and live-audio viewport gates at five sizes pass. Final static review reports zero remaining P0/P1 findings in this tranche.
- Native smoke evidence: the current-source VJ Desk was inspected maximized in the 1920x1032 Windows work area, reached Ready without a command mismatch, and started/stopped the system-default 48 kHz WASAPI shared microphone. The sampled callback was 480/480/480 frames, capture-to-worker current/maximum 0.1/10.1 ms, overrun 0/0, and queue 0/1/4. This is a single stopped/silent-microphone smoke run, not an accuracy or endurance result.
- Pending: arbitrary 16-band Audio Reactive Rack mappings, kick/snare separation, spectral centroid/density, ASIO, post-open applied-config reporting, physical latency, one-hour hardware soak, and matched TouchDesigner trials.

This status is an implementation inventory, not an acceptance pass. Check off performance and parity gates only when their recorded evidence is added below.
