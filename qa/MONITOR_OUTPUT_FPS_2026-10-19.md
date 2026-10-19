# Monitor and output frame-rate acceptance — 2026-10-19/20

Task 17, shared `main`, base `ec399e9070452b888b46bd9f7e70f87818ca7245`.
All task changes remain uncommitted for review. No product version change.

## Summary

- The owner's 14 fps observation was the in-app monitor (budget 10 fps Program / 5 fps Preview), not the real output.
- Real output was measured first. It was **below target**: 57.7-58.6 fps at a 60 Hz Display target, with 95-265 ms stalls at every video loop wrap. Both were fixed (below).
- Monitors now run Smooth 30 fps (Program, staged/active Preview) / 15 fps (passive Preview) by default, with monitor-only adaptive protection. Measured on this machine: 28-29 fps per bus with one layer, 25-26 fps with three layers and two outputs, with output cadence unchanged.
- The Output pane shows per-output delivered fps and the missed intervals of the last 10 s (the cumulative total is in the tooltip; zero is shown neutral, more than 6 in the window uses the warning tone; the first sample of an output is the baseline, so startup never counts). Monitor footers read "Monitor 28.7fps" / 「モニター 28.7fps」. A short "Protecting output" note appears when monitors are throttled.
- Settings menu: Monitor quality (Smooth / Balanced / Low CPU), persisted in `syndocal.monitorQuality.v1`.

## Measurement method and boundaries

- Release build, QA fixtures only (copies of `target/qa/vj-perform-ui/vj-perform-final.sdc` with H.264 `h264_1080p30.mp4`, 1920x1080, 30 fps, 12 s loop; DMX routes disabled). Display is an explicit "QA Task17 Display" window on the primary screen, never a projector. A "QA Task17 Spout" sender is the second output where noted.
- Output telemetry (`get_live_video_output_health`) records successful Display `present()` completion or successful Spout/NDI SDK submission. It does not assert physical scanout or receiver delivery. Each worker owns a fixed 600-timestamp atomic ring; no frame-loop locks, copies, allocation, threads or I/O were added.
- `droppedFrames` is the high-water deficit between elapsed 60 Hz slots and completed submissions (an estimate of missed intervals, not a hardware or receiver counter). Targets are 60 Hz even for the 30 fps source.
- Independent measurement: PresentMon 2.6.0 on the app process (`MsBetweenPresents`, displayed-fps), CPU from process-tree cumulative CPU time, GPU from per-process GPU engine counters (sums of engine utilisation, not normalised whole-GPU percent). 25 s windows after a 10 s warm-up. Machine share uses 32 logical CPUs.
- Monitor fps is counted in the WebView (unique `<img>` loads or newly drawn canvas bitmaps), i.e. frames the operator actually sees.
- Before = the original release executable (SHA256 `049BBEA9...592C`, built from the base). After = release executable `215736756A9D...9F75` from this tree.
- The original build could not arm Display + Spout together (`arm` rejected `publication_failed`, see "Findings"), so before/after comparisons use Display-only fixtures. Display + Spout is measured on the new build only.

## Results: Display only (same fixture, before vs after)

PresentMon present-to-present intervals; hitches are intervals over 25 ms.

| Layers | Build / monitor setting | Display fps (OS displayed) | Present p50 / p95 / p99 / max ms | Hitches >25 ms | Monitor Program / Preview fps | CPU (one core % / machine %) | GPU 3D mean % |
| --- | --- | ---: | --- | --- | --- | --- | ---: |
| 1 | Before, 10/5 budget | 58.63 | 16.95 / 17.85 / 18.47 / 95.9 | 2 (96, 95 ms at loop wrap) | 10.7 / 7.0 | 142 / 4.4 | 7.9 |
| 1 | After, Low CPU (10/5 budget) | 60.02 | 16.68 / 17.58 / 18.20 / 28.6 | 2 (29, 29 ms) | 9.4 / 4.9 | 124 / 3.9 | 7.0 |
| 1 | After, Smooth (30 fps) | 60.02 | 16.65 / 17.68 / 18.39 / 29.9 | 2 (30, 30 ms) | 29.0 / 29.1 | 175 / 5.5 | 9.1 |
| 3 | Before, 10/5 budget | 57.75 | 16.94 / 19.29 / 20.23 / 265.1 | 6 (258, 265 ms and 4 of 26-37 ms) | 10.2 / 5.6 | 175 / 5.5 | 13.2 |
| 3 | After, Low CPU (10/5 budget) | 60.04 | 16.65 / 18.93 / 19.41 / 36.2 | 2 (36, 36 ms) | 9.5 / 4.9 | 173 / 5.4 | 14.0 |
| 3 | After, Smooth (30 fps) | 60.02 | 16.61 / 19.16 / 19.84 / 40.3 | 3 (37, 25, 40 ms) | 28.5 / 28.9 | 246 / 7.7 | 15.6 |

Native telemetry for the after runs (successful presents): 59.95-60.03 fps, rolling p99 18.2-20.0 ms, zero missed-slot deficit growth in every 25 s window.
Monitors at the new target (Smooth) cost about +50 to +70 percent of one core (1.5 to 2.3 percent of this 32-thread machine) and about +1.5 GPU 3D points; output cadence is unchanged between Low CPU and Smooth (p50/p95/p99 within 0.5 ms).

## Results: Display + Spout (new build only)

| Layers | Monitor setting | Display / Spout delivered fps | Native p50 / p95 / p99 ms (Display) | Monitor Program / Preview fps | CPU one core % / machine % | GPU 3D mean % |
| --- | --- | --- | --- | --- | --- | ---: |
| 1 | Low CPU | 60.03 / 59.98 | 16.6 / 17.9 / 18.9 | 9.4 / 5.0 | 181 / 5.7 | 7.2 |
| 1 | Smooth | 60.01 / 60.01 | 16.6 / 18.0 / 19.0 | 28.3 / 28.7 | 250 / 7.8 | 8.6 |
| 3 | Low CPU | 59.95 / 59.99 | 16.3 / 19.7 / 20.3 | 9.4 / 4.9 | 457 / 14.3 | 12.8 |
| 3 | Smooth | 59.98 / 60.02 | 16.3 / 19.9 / 20.9 | 24.9 / 26.1 | 545 / 17.0 | 12.4 |

Display present p99 with three layers: 20.3 ms (Low CPU) and 20.9 ms (Smooth); worst PresentMon interval 44.5-46.6 ms (loop wrap with six decoders). Neither monitor bus was in "Protecting output" at the end of any run; brief protection during the first seconds after enabling layers is expected.

## Findings and fixes (output first)

1. **Output cadence deficit (fixed).** Every output loop slept `target - elapsed` after each frame, so scheduler overshoot accumulated: 58.6 fps and growing missed-slot count at a 60 Hz target. `video_output_pacer.rs` keeps an absolute deadline per output (Display, Spout, NDI): overshoot is compensated, overdue slots are retired arithmetically (no catch-up burst), slow frames are never delayed further. Result: 60.0 fps, p50 16.65 ms.
2. **Video loop-wrap stall (fixed).** A backward jump (every loop wrap) re-opened the file and recreated the hardware decoder inside the output loop: 95-121 ms per layer (up to 265 ms with three layers). `libav_decoder.rs` now rewinds the open session in place (seek + decoder flush, other inputs' look-aheads keep running). A rewind whose seek fails (raw elementary streams, image sequences, unseekable inputs, or a start time that cannot seek to 0) is an explicit failure that falls back to a fresh `open_libav_session` at any position, so a late frame is never served as position 0 and an already-drained decoder never errors every wrap. Tests: software real-wrap tests (B-frame MP4, positive/negative start-time TS) with 50 wraps before and after EOF, pixel parity against a fresh decoder, `opens == 1`, no stale drained/EOF state; an injected seek failure before and after EOF proving the re-open and the correct frame; a real-GPU 50-wrap test (`real_gpu_loop_wraps_...`). `resets` now counts in-place rewinds and `opens` no longer rises on a wrap; both are read only by tests and the QA benchmark, never by runtime thresholds. No periodic hardware re-open was added: it would reintroduce a 100 ms stall on a schedule, errors already fall back to a fresh open, and 50 hardware wraps pass without a hardware error. The negative start-time TS fixture falls back to a re-open on each wrap (correct frames, old cost). Output stall at wrap: 95-265 ms to 29-47 ms. Monitor seek render (debug benchmark): p50 131.7 to 54.0 ms, opens 8 to 2.
3. **Output startup race (fixed).** Spout/NDI workers rendered before Arm completed its Active-to-Ready transition and treated their own startup fence as a fault (Arm with Display + Spout: `publication_failed`; earlier Display open attempts: `stale_fence`). `OutputOwnershipActivation::wait_until_ready` (engine, one-time barrier, never in the frame loop, 30 s bound, cancellation and invalidation fail closed) holds the prepared sender idle until that exact activation is Ready. A readiness timeout or invalidation is reported through the worker failure path and the console; the worker does not raise a safety fence itself, because Arm owns that transition and a fence from the worker could race with it.
4. **Startup deficit (fixed after review).** The Display creation-thread first present is recorded before the shared start gate; `restart_cadence()` re-anchors the missed-slot baseline at the paced render loop so startup is not reported as dropped frames.
5. **Monitor protection over-triggered (fixed).** With the pre-pacer build, one missed slot per second kept the policy permanently active (monitors fell to 3 fps at "Low CPU"). A burst of three or more missed slots in one poll is now required; with the pacer the deficit no longer grows in steady state.

## Transport decision

Real WebView2 binary Tauri IPC, decoding and drawing were measured in the original release executable with three playing H.264 layers and a separately staged Preview, 50 frames per bus (100 total), source rendering included. Variants ran in sequence, so cold-start effects appear in p99 and the first `<img>` mean.

| Transport / presentation | Bus requests | Mean total ms | p95 total ms | p99 total ms | 100 frames wall ms | Typical packet bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| JPEG / `<img>` blob | Serialized | 16.29 | 22.5 | 123.4 | 1630.2 | 8256 |
| JPEG / bitmap + canvas | Serialized | 9.19 | 12.8 | 17.7 | 920.4 | 8541 |
| RGBA / canvas | Serialized | 9.36 | 13.8 | 16.1 | 936.4 | 230440 |
| JPEG / bitmap + canvas | One in flight per bus | **8.74** | **12.1** | **13.7** | **493.6** | 8679 |
| RGBA / canvas | One in flight per bus | 10.90 | 15.7 | 24.0 | 644.9 | 230440 |

Chosen: JPEG quality 68 over the existing `SYLV` v2 binary IPC, decoded with `createImageBitmap` and drawn to canvas, with independent single-flight Program and Preview requests. Raw avoids JPEG encode/decode but moves about 26 times more bytes and loses on measured end-to-end latency. No new packet version, fallback transport, or shared frame consumer. Evidence: `target/qa/task17/native/ipc-benchmark-playing-before.json` (paused comparison: `ipc-benchmark-before.json`); browser-only decode: `target/qa/task17/browser-decode-benchmark.json`.

Debug Rust benchmark `live_video_monitor_benchmark_raw_vs_jpeg_two_real_outputs` (render/packet/encode only, excludes IPC, WebView and output; debug build, so absolute values only compare like with like): before sequential render p50/p95 6.06/12.10 ms, JPEG encode 6.75/9.37 ms; after 4.16/5.16 ms and 4.80/5.28 ms (machine state differs between runs; the monitor packet code itself is unchanged).

## Scheduling and preferences

- Smooth: Program 30 fps; staged/active Preview 30 fps, otherwise 15 fps. Balanced: 15/15; Low CPU: today's 10/5 budget. Preference `syndocal.monitorQuality.v1`, synchronised across windows; Smooth is the initial value, adaptive protection lowers it when the machine cannot sustain the output.
- Each bus has at most one request/decode in flight. Missed slots are discarded, never replayed. Source identity, explicit Preview transport generation, visibility and disposal fence stale responses. Equal snapshot publications cannot restart cadence.
- Pressure (busy drops, repeated expensive render/encode, output p95 above 1.35x budget or 20 percent above its own baseline, a burst of 3+ missed slots per poll, stale last-frame age) raises monitor-only tiers 75/50/25 percent. Healthy recovery takes five seconds per tier. Severe stalls pause monitor IPC for one second with staggered probes; telemetry failure never restores full load. Output pacing and settings are never reduced.
- Preview keeps its dedicated renderer/transport and stays Preview-only; neither monitors nor telemetry consume a shared latest-frame queue. Program, recording and external output routing are unchanged.

## Limitations

- Output numbers are for this machine (32 threads, one primary display at 60 Hz) and a 1080p30 H.264 clip. Output counters are native submit/send boundaries, not scanout or receiver proof. Independent PresentMon observation covers only the Display swapchain of the app process.
- The new target is 30 fps; measured 28-29 fps with one layer and 25-26 fps with three layers plus two outputs, because the monitor bus shares CPU with the renderer. This is expected: the output keeps priority.
- Loop wrap still costs one late present (29-47 ms) because the clip restarts from its first keyframe.
- Enabling layers while outputs run still opens their decoders (existing behaviour); it produces a one-off stall at that moment and the monitor protection tier engages briefly.
- Display open via the QA control path reports `rejected internal` although the window opens and delivers frames. Observed identically on the original executable; not changed here.
- Spout receiver-side delivery is not measured. `spout_receiver_cadence_tests.rs` is an ignored, explicit QA tool (polling readback; its numbers are not output fps).
- NDI uses the same pacer and health hook but no NDI receiver or hardware was exercised.

## Validation

See the final report (`sol-handoff/task17-smooth-monitors-output-fps-result.md`) for the validation table, review disposition and screenshot list. Native evidence: `target/qa/task17/native/` (`final-summary.md/json`, `*-presentmon.csv`, `*-usage.json`, per-run JSON), screenshots `target/qa/task17/perform-ja-1366x768-monitor-output-health.png` and `perform-en-1920x1080-monitor-output-health.png`.
