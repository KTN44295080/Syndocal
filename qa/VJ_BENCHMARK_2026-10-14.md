# VJ render benchmark — 2026-10-14

Owner request (2026-10-14): measure Syndocal's VJ output performance against
SynapseRack. This records Tier A (Syndocal alone, headless, the native output's
render seam) and what it changed. No same-machine SynapseRack numbers exist
yet; Tier C below is what that comparison needs. The operator-task comparison
remains `qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md`.

## Tiers

| Tier | What | State |
| --- | --- | --- |
| A | Decode + CPU composite + effects through the native output's seam, headless, per-frame timing | Done (this file) |
| B | The same scenes in the running app with real output windows: presentation, the output's 600-frame deadline window, NDI/Spout | Not run |
| C | SynapseRack and Syndocal on one machine, the same media and layer counts, frame times captured externally (PresentMon) | Not run: needs the owner to approve installing a pinned SynapseRack build and PresentMon |

## Rig and media

- **Machine.** AMD Ryzen AI MAX+ 395 (16 cores / 32 threads) with Radeon 8060S
  integrated graphics, 56 GB visible RAM, Windows 11. This is not the
  production rig named in the operator benchmark (RTX 5090 / 13900KF).
- **Media pack.** `qa/harnesses/New-VjBenchMedia.ps1` produces 12 s FFmpeg
  `testsrc2` clips with a burnt-in frame counter: H.264 1080p30, 1080p60 and
  2160p30, HEVC 1080p30, ProRes 422 1080p30, HAP and HAP Q 1080p30. It writes
  `manifest.json` with each file's SHA-256 (for example `h264_1080p30.mp4`
  `53861258…0c2b`, `hap_1080p30.mov` `e6a0b9bc…ec81`).
- **Harness.** `app/src-tauri/src/tests/vj_benchmark_tests.rs`
  (`vj_benchmark_tier_a`, ignored). It calls
  `prepare_output_artistic_render_result_preview_with_effects_and_transitions`,
  the seam the native output window renders through, with the production
  decoder chain (libav with hardware decode, then the CLI fallback).
- **Timing.** 30 warm-up frames, then 240 measured frames at a 60 fps media
  clock. The budget is 16.7 ms (60 fps).
- **Layer scenes.** The base layer is Normal at full opacity. Every other layer
  is at 50 % opacity, cycling through Add, Screen and Multiply, so it is really
  blended.
- **Running it.**
  `SYNDOCAL_VJ_BENCH_MEDIA=<dir> cargo test --release -p syndocal --bin syndocal vj_benchmark -- --ignored --nocapture`.
  Optionally set `SYNDOCAL_VJ_BENCH_ONLY=<parts>` to run only some scenarios.
  Results go to stderr and `target/qa/vj-bench/<unix ms>.json`.

## Finding: the live output composited blended layers on a slow CPU path

The native output composites on the CPU, and the GPU only maps and presents
the result (`present_output_artistic_result`).
- **Fast path.** It existed only for an opaque layer, Normal blend, at full
  opacity.
- **Everything else.** Any translucent or Add/Screen/Multiply layer took the
  generic per-pixel sampler, with colour and FX arithmetic and a `roundf`
  library call per channel.
- **Cost.** About 100 ms per 1080p layer on one thread, so a second visible
  layer cost about 140 ms.

The fix (`crates/video/src/lib.rs`) keeps the output byte-identical:
- **Wider fast path.** It now covers any opacity and blend mode, as long as
  colour/FX are default and there is no rotation. The generic path is still
  used whenever an adjustment, a rotation or a non-finite transform is
  present.
- **Exact rounding.** `round_clamp_u8` gives exactly
  `round().clamp(0, 255) as u8` for all 2^32 `f32` bit patterns. An exhaustive
  test covers this.
- **Lookup tables.** On layers of 1<<17 pixels or more, opaque source pixels
  use 64 KiB tables built from the same blend functions.
- **Covered rows only.** Only the rows a layer covers are visited, and an
  off-screen layer returns at once.
- **Row bands.** A layer covering 1<<18 pixels or more is split into up to
  8 row bands on rayon's global pool, which starts its threads once and is
  shared by every output.
- **Why the pool.** The independent review measured per-frame scoped
  threads at about 0.57 ms per layer to start and join, plus a Windows
  loader-lock stall. The pool replaced them.
- **Known one-off cost.** The pool starts its threads on the first large
  blend: 32 threads here, a few milliseconds once. Warming it when an
  output starts is a small follow-up.

| Layer blend, release, one call (1 thread / 8 bands) | Before | After |
| --- | ---: | ---: |
| 1080p, Normal, opaque, full opacity | 2.4 / — ms | 1.4 / 0.46 ms |
| 1080p, Add at 50 % (generic before; fast path and table after) | 98 / — ms | 3.9 / 1.0 ms |
| 4K, Add at 50 % | 408 / — ms | 13.6 / 3.3 ms |

These come from `axis_aligned_composite_release_cost`, measured on the same
machine. The layer there covers 71 % of the rows.

## Results (frame time in ms; "over" is frames over 16.7 ms, out of 240)

Before = commit `e4613dfb` code, measured through the preview seam (the same
CPU composite plus an identity output mapping). After = this change, measured
through the native output seam.

| Scenario | Before p50 / p95 | After p50 / p95 | After over 16.7 ms |
| --- | ---: | ---: | ---: |
| H.264 1080p30, 1 layer | 10.3 / 19.2 | 8.0 / 17.2 | 31 |
| H.264 1080p60, 1 layer | 21.3 / 24.3 | 15.8 / 17.0 | 27 |
| HEVC 1080p30, 1 layer | 10.8 / 23.8 | 8.0 / 17.6 | 58 |
| ProRes 422 1080p30, 1 layer | 11.4 / 25.2 | 8.3 / 17.0 | 23 |
| HAP 1080p30, 1 layer | 10.8 / 18.2 | 8.1 / 13.0 | 0 |
| HAP Q 1080p30, 1 layer | 10.9 / 30.2 | 8.3 / 20.8 | 120 |
| H.264 1080p30 × 2 | 155.9 / 180.4 | 13.0 / 32.0 | 120 |
| H.264 1080p30 × 4 | 439.9 / 490.0 | 23.5 / 62.2 | 240 |
| H.264 1080p30 × 8 | 1003.7 / 1098.6 | 46.9 / 124.2 | 240 |
| HAP 1080p30 × 2 | 145.8 / 167.3 | 12.3 / 22.9 | 120 |
| HAP 1080p30 × 4 | 414.8 / 455.7 | 24.2 / 44.8 | 240 |
| HAP 1080p30 × 8 | 959.0 / 1029.8 | 45.8 / 88.6 | 240 |
| H.264 2160p30, 1 layer, 4K out | 43.9 / 84.7 | 29.7 / 59.8 | 240 |
| 4 × H.264 1080p30 into 4K out | 1719.1 / 1813.0 | 86.6 / 185.2 | 240 |

- **One layer.** A single HAP 1080p layer now holds 60 fps (0 of 240 frames
  over budget). A single H.264, HEVC or ProRes layer misses only on some
  frames that decode.
- **Two or more blended 1080p layers.** p50 fits 60 fps up to two layers.
  Frames that decode still miss. Multi-layer scenes are 12–21× faster.
- **Raw JSON.** `target/qa/vj-bench/1791978690933.json` (before) and
  `1791982077048.json` (after). `1791980077318.json` is an intermediate run
  with per-frame threads.

## Where the remaining time goes

A temporary probe was removed after measuring. It was taken on the
intermediate per-frame-thread build. It split the 8-layer 1080p H.264
frame into two phases:

| Phase | Frames with no new media frame | Frames where every layer decodes |
| --- | ---: | ---: |
| Frame preparation (fetch/decode, all layers) | ≈ 27 ms | ≈ 90–135 ms |
| Composite (all layers) | ≈ 29 ms | ≈ 29 ms |

1. **Decode runs synchronously on the render thread, one layer after another.**
   A 30 fps clip decodes on every other 60 fps frame, which gives the bimodal
   p50/p95. A single 1080p layer already exceeds 16.7 ms on its decode frames.
   Next step: decode each layer ahead of time on its own worker thread and
   hand the render thread a ready frame.
2. **There is about 3.4 ms per layer of preparation even when no frame is
   decoded.** Every tick clones each layer's 8 MB RGBA frame, and HAP's DXT
   frames are expanded to RGBA on the CPU on every composite. Next steps:
   share frames by reference counting instead of copying, and keep the
   decoded RGBA result of a DXT frame.
3. **Compositing was about 3.6 ms per layer** in that probe, including the
   output buffer allocation and the blend. The pool and covered-rows change
   lowered it again; the 8-layer p50 dropped by 21 ms between the two runs. Moving the composite onto the GPU would remove
   it from the CPU budget entirely. A wgpu compositor that accepts BC/DXT
   textures already exists (`GpuCompositor`, `GpuSurfacePresenter`
   `present_prepared_output`), but the live path uses the CPU artistic seam.
   That is a design decision for the owner, not a tuning step.

## Follow-up 2026-10-15: frame copies and parallel decode

Two of the next steps above are done.

1. **No more per-tick frame copies (`e53642ed`).**
   - **Before:** each tick copied every layer's 8 MB frame three times. The
     libav decoder cloned its cache entry to inspect it and again to return
     it, and composition copied the frame out of the frame queue.
   - **Now:** the frame queue shares frames (`Arc`). The decoder reads its
     cache in place, so a reused frame is copied once.
2. **A tick's libav frames decode in parallel.**
   - **What runs in parallel:** on the live seam, the renderer first hands
     the provider every input the tick needs. The libav decoder decodes the
     distinct file inputs on separate sessions on the rayon pool.
   - **What stays in input order:** the cache, session and fallback
     decisions, so the results and counters equal decoding one by one
     (`libav_parallel_decode_tests.rs`).
   - **Where it is off:** the cancellable recording path and HAP inputs
     still decode one by one.

Results on the same machine and media, through the production decoder
(`NdiAwareVideoFrameDecoder`, CLI fallback off;
`target/qa/vj-bench/1791990134390.json`, re-run `1791991646115.json`):

| Scenario | 2026-10-14 after (p50 / p95) | Now (p50 / p95) | Now over 16.7 ms |
| --- | ---: | ---: | ---: |
| H.264 1080p30, 1 layer | 8.0 / 17.2 | 5.7 / 15.0 | 0 |
| H.264 1080p60, 1 layer | 15.8 / 17.0 | 13.9 / 15.0 | 0 |
| HEVC 1080p30, 1 layer | 8.0 / 17.6 | 5.6 / 15.7 | 2 |
| ProRes 422 1080p30, 1 layer | 8.3 / 17.0 | 5.5 / 15.4 | 0 |
| HAP 1080p30, 1 layer | 8.1 / 13.0 | 6.1 / 11.1 | 0 |
| HAP Q 1080p30, 1 layer | 8.3 / 20.8 | 5.8 / 18.8 | 109 |
| H.264 1080p30 × 2 | 13.0 / 32.0 | 7.4 / 18.5 | 116 |
| H.264 1080p30 × 4 | 23.5 / 62.2 | 14.4 / 28.3 | 120 |
| H.264 1080p30 × 8 | 46.9 / 124.2 | 26.3 / 46.5 | 240 |
| HAP 1080p30 × 2 | 12.3 / 22.9 | 7.6 / 14.3 | 0 |
| HAP 1080p30 × 4 | 24.2 / 44.8 | 16.2 / 23.1 | 120 |
| HAP 1080p30 × 8 | 45.8 / 88.6 | 25.9 / 39.8 | 240 |
| H.264 2160p30, 1 layer, 4K out | 29.7 / 59.8 | 24.0 / 54.3 | 232 |
| 4 × H.264 1080p30 into 4K out | 86.6 / 185.2 | 45.4 / 83.5 | 240 |

**What holds 60 fps now:**
- **1080p, one layer:** every codec except HAP Q.
- **HAP, two layers:** holds 60 fps.
- **H.264, two layers:** misses only on some decode frames (p95 18.5 ms).
- **Four layers:** p50 fits 60 fps, but half the frames miss.

**Remaining costs:**
- the per-tick DXT-to-RGBA expansion for HAP;
- HAP decode, which is not yet parallel;
- the decode itself on decode frames;
- 4K compositing on the CPU.

### Where the time goes now (8 × H.264 1080p, temporary probe, removed)

| Phase, per frame | Frames with no new media frame | Frames where every layer decodes |
| --- | ---: | ---: |
| Decode ahead (all layers, parallel) | ≈ 8.5 ms | ≈ 28 ms |
| Frame-queue push | ≈ 2.5 ms | ≈ 2.5 ms |
| Composite | ≈ 10 ms | ≈ 10 ms |

**Where the time goes when no new media frame arrives.** The decoders still
return each layer's frame as a fresh 8 MB copy, which the queue then
replaces. That allocate, page-fault, copy and free cycle (about 1.3 ms per
layer) is the cost.

**What was tried and reverted.** A per-frame cache of the DXT-to-RGBA
conversion for HAP made no measurable difference. The conversion is not a
material cost, so the change was reverted.

**The next gains are design changes, not tuning:**
1. **Shared pixel buffers.** `VideoFrame.data` would become reference
   counted, so a reused frame is not copied and its buffer is not freed and
   reallocated every tick. This removes most of the ≈ 11 ms on frames with
   no new media frame (8 layers).
2. **Asynchronous decode one frame ahead.** Each layer decodes its next
   media frame between ticks, so a decode no longer lands on the frame that
   needs it. This flattens p95 toward p50.
3. **GPU composition for the live output.** It takes the remaining ≈ 1.25
   ms per 1080p layer, and far more at 4K, off the CPU.

## Follow-up 2026-10-15 (2): shared frames on the live path

**What changed.** Design step 1 above is done without changing `VideoFrame`.
- **Decoder.** It gains shared variants (`decode_input_frame_shared`,
  `decode_input_frames_shared`). The libav cache holds `Arc<VideoFrame>`,
  so a cache hit or reused frame is handed out without a copy.
- **Provider.** `frames_for_layer_shared` is added.
- **Renderer.** On the live (uncancellable) path, a layer without clip or
  layer effects queues the decoder's own frame. A layer with effects still
  copies it once to apply them.

**What does not change.** The owned API, recording and every other caller
behave as before.

| Scenario (1080p) | Before (p50 / p95) | Now (p50 / p95) | Now over 16.7 ms |
| --- | ---: | ---: | ---: |
| H.264, 1 layer | 5.6 / 15.8 | 4.5 / 14.7 | 2 |
| H.264 × 2 | 7.4 / 18.5 | 5.9 / 16.3 | 3 |
| H.264 × 4 | 14.4 / 28.3 | 8.8 / 23.9 | 120 |
| H.264 × 8 | 26.3 / 46.5 | 15.1 / 35.7 | 120 |
| HAP × 2 | 7.6 / 14.3 | 4.8 / 11.9 | 0 |
| HAP × 4 | 16.2 / 23.1 | 9.0 / 18.2 | 56 |
| HAP × 8 | 25.9 / 39.8 | 15.5 / 29.3 | 120 |
| H.264 2160p, 1 layer, 4K out | 24.0 / 54.3 | 17.3 / 48.3 | 121 |
| 4 × H.264 into 4K out | 45.4 / 83.5 | 24.7 / 64.4 | 240 |

**What these numbers mean:**
- **Frames that only re-show a frame** now fit 60 fps up to 8 layers of
  1080p (8 × H.264 p50 15.1 ms).
- **Two layers of H.264 or HAP** hold 60 fps.
- **The frames that still miss** are the ones where several clips advance
  at once: the 120 of 240 frames that decode. That is design step 2
  (asynchronous decode one frame ahead).

## Follow-up 2026-10-15 (3): decoding the next frame ahead

Design step 2 is done for libav (H.264, HEVC, ProRes).

**How decoding ahead works.**
- On the live path, after a layer is served frame F, its session decodes
  the next frame on the decode pool, between frames.
- The next request that lands on that frame takes it without decoding.
  F's time is less than the request's time, and the request's time is at
  most the next frame's time.
- Paused layers, steps back, seeks and loops fall through to the on-demand
  decision exactly as before. The session's `last_request_ms` is restored
  after the look-ahead.
- If the clip ended or the look-ahead failed, the session is dropped and
  reopened on demand.
- Recording and other owned callers never decode ahead.
- A 150-tick playback script returns the same frames as on-demand decoding
  (`libav_parallel_decode_tests.rs`).

**Measurement change.** The benchmark now starts frames on a 60 Hz cadence,
as the output's frame clock does, so work between frames overlaps idle time
as it would in the app. Only each frame's render is timed;
`SYNDOCAL_VJ_BENCH_UNPACED=1` restores back-to-back rendering. Both columns
below are paced.

| Scenario (paced 60 Hz) | Without look-ahead (p50 / p95, over) | With look-ahead (p50 / p95, over) |
| --- | ---: | ---: |
| H.264 1080p, 1 layer | 5.3 / 14.6, 1 | 3.7 / 4.3, 0 |
| H.264 × 2 | 4.9 / 16.2, 6 | 4.1 / 5.0, 0 |
| H.264 × 4 | 7.9 / 22.6, 120 | 7.7 / 9.3, 0 |
| H.264 × 6 | 11.2 / 29.5, 120 | 12.6 / 14.8, 1 |
| H.264 × 8 | 14.4 / 35.5, 120 | 17.5 / 20.7, 126 |
| HAP × 4 | 7.9 / 17.2, 24 | 7.9 / 10.4, 0 |
| HAP × 6 | 14.0 / 22.8, 120 | 12.5 / 15.9, 3 |
| HAP × 8 | 16.6 / 28.7, 120 | 14.8 / 21.8, 120 |
| H.264 2160p, 1 layer, 4K out | 16.4 / 48.4, 120 | 17.5 / 32.1, 122 |
| 4 × H.264 into 4K out | 24.7 / 64.3, 240 | 29.8 / 32.9, 240 |

Look-aheads were used on about 99 % of requests, and none was discarded in
these scenes.

**What holds 60 fps now.** Every 1080p codec with up to 4 layers holds 60 fps
on this machine (0 of 240 frames over), and 6 layers come within 1–3
frames.

**Where it falls short:**
- At 8 layers the look-ahead decodes compete with the frame being rendered,
  so p50 rises while p95 falls.
- 4K output is bound by CPU composition, which is design step 3.

**Not covered.** HAP inputs do not decode ahead (the HAP decoder is separate).
They benefit from the shared frames instead.

## Follow-up 2026-10-15 (4): GPU composition for large Display outputs

Design step 3 is done for the native Display output only, as the owner
decided: 4K and many-layer outputs.

**When the GPU composites.**
- **Size:** the output is 4K UHD or larger with two or more layers, or it
  has six or more layers. Below that the CPU is fast enough.
- **Content:** the composition is plain:
  - no effect chain executes (or fails to resolve) in any scope;
  - no transition bus or clip transition is active;
  - no layer is rotated or has a colour adjustment or effect;
  - frames are RGBA, BGRA or HAP DXT1/DXT5.
- **Everything else renders on the CPU as before.** That includes every
  blackout, every failure, HAP Q and HAP R, and the first frame before the
  window's GPU is known.
- **Other outputs are unchanged.** Recording, NDI, Spout and previews still
  composite on the CPU.

**What the presenter does.**
- **Upload.** It uploads each layer's frame into a storage buffer, followed
  by that layer's sampling lookup. A frame the slot already holds is not
  uploaded again.
- **HAP.** DXT1/DXT5 frames upload as they are, compressed. The shader
  decodes them with the CPU decoder's integer arithmetic. No BC texture
  support is needed.
- **Present.** It composites, maps and presents on the GPU.
- **Releasing frames.** When the output next renders on the CPU, the
  presenter drops the composition slots, releasing their storage buffers
  and the frames they held (including when a blackout takes over).

**Safety.** The guarantees of the artistic seam hold on the new path.
- **Admission is the same.** The GPU composition carries the same evidence
  and passes the same fail-closed admission before the surface is touched.
  The frame must be fresh and error-free, for this project epoch and this
  output. The mapping identity is derived again and must match.
- **Admission is stricter.** It also refuses any layer the GPU would not
  composite as the CPU does.
- **Blackouts are unchanged.** A blackout never becomes a GPU composition.
  It stays the payload-less exact-black clear.
- **Display control is unchanged.** The authority fence before every
  present and the Follow settlement are as before.
- **Device limits.**
  - A layer larger than the device can bind (frame plus lookup) keeps the
    output on the CPU.
  - If the device cannot allocate a layer, the presenter composites that
    frame on the CPU exactly as the artistic path would and presents it.
    That window then stays on the CPU.
- **Prepared frames are reused.** Stage faults and failed frame checks
  (HAP Q, HAP R, oversized layers) reuse the prepared frames on the CPU
  without decoding again. A decoder error makes `prepare_frames` return
  `Err`, so the CPU path prepares frames again.

**The GPU output matches the CPU output.**
- **The first version did not.** The tests and the independent review
  found two defects in it:
  - **Sample positions.** They were recomputed in GPU floating point. Where
    a sample fell on a pixel boundary, it took the neighbouring source
    pixel: 1.7 % of pixels for a 90° rotation, and whole rows for some
    scales and crops.
  - **HAP decoding.** It used the GPU's hardware BC decoder. That decoder
    interpolates differently from the CPU's integer decoder, and the
    difference varies between vendors.
- **What changed.**
  - **Sample positions.** For each output column and row, the presenter
    reads which source pixel to sample. The CPU compositor computes those
    indices with the same code it uses itself (`AxisAlignedSampling`).
  - **HAP decoding.** The shader decodes DXT blocks with the CPU decoder's
    integer formulas.
- **The result.**
  - **Sampling.** The GPU samples exactly the CPU's pixels.
  - **Decoding.** Every opaque decoded HAP pixel is identical.
  - **Rounding.** Output differs only where the GPU rounds a blend the
    other way: at most one level per blend. The tests saw one level, in
    well under 0.1 % of channels.
  - **Tests.**
    - `gpu_surface_composition_tests.rs`: read back without a window.
    - `gpu_display_composition_tests.rs`: eligibility, fallback and
      admission.
    - The app test `native_display_composites_a_large_plain_output_on_the_gpu_until_blackout`.
- **Rotated layers stay on the CPU.**

**Measured.**
- **Setup:** release build on this machine, synthetic opaque frames, no
  decode. Clips advance every other tick (30 fps media on a 60 Hz output).
- **CPU path:** composite, keep the last-valid copy, upload and map.
- **GPU path:** upload the changed layers, then composite and map.
- Both are timed until the GPU finishes.
- Command:
  `cargo test --release -p video --features libav --lib gpu_composition_benchmark -- --ignored --nocapture`

| Scene | CPU path (p50 / p95 ms) | GPU path (p50 / p95 ms) |
| --- | ---: | ---: |
| 4K out, 2 × 4K RGBA | 14.6 / 15.9 | 5.4 / 6.3 |
| 4K out, 4 × 4K RGBA | 24.1 / 26.6 | 9.5 / 11.2 |
| 4K out, 8 × 4K RGBA | 43.3 / 46.9 | 18.6 / 21.5 |
| 4K out, 4 × 1080p HAP (DXT1) | 43.7 / 45.9 | 2.5 / 3.0 |
| 1080p out, 6 × 1080p RGBA | 11.0 / 12.0 | 3.0 / 4.1 |
| 1080p out, 8 × 1080p HAP (DXT1) | 53.5 / 59.8 | 1.4 / 1.8 |

**What this means.**
- **Composition is no longer what limits 4K output.** At 4K, composition
  and presentation of four layers now fit a 60 Hz frame with room for
  decoding.
- **The VJ benchmark above is unchanged.** It times the renderer without a
  window, so it still composites on the CPU and does not show this change.
- **The largest GPU gain is HAP.** Its compressed frames upload as they are,
  without being expanded on the CPU.

**Not yet checked.**
- A 4K Display output on real hardware, with a window, has not been run.
  The headless tests run the same encoding code the presenter submits.
- The CPU fallback after an allocation failure needs a window, so no test
  runs it.

**Review fixes and validation (2026-10-15).**
- Checkout: `main`, base `fe4a7d01f3596576907c356821989a7cf2e2a7a6`.
  The change was independently reviewed by Claude before commit.
- Output-buffer and composition-slot allocation now catch both out-of-memory
  and validation errors, including bind groups made from invalid buffers.
  Failed output buffers are discarded before returning an error; failed
  composition slots are dropped before the CPU fallback disables GPU composition.
- The upload-cache regression now corrupts a forgotten upload and verifies
  pixel-correct re-upload without reallocating the slot.
- MSVC-wrapper tests: `test -p video --features libav` **229 passed, 0 failed,
  10 ignored**; `test -p video` **205 passed, 0 failed, 5 ignored**.
- `test -p syndocal --bin syndocal`: **2104 passed, 2 failed, 22 ignored**.
  The failures were
  `project_backup_listing_missing_directory_pagination_exact_bytes_and_no_allocation`
  and `receiver_disconnect_returns_child_to_quarantine_then_retry_reaps_it`
  (reaper timeout). Each passed alone (**1 passed, 0 failed** each).
  Neither test's implementation was changed.
- `build-nobundle.cmd`: exit **0**. `native-gate.ps1` against this checkout's
  release executable: exit **0**, two visible titled windows (`Syndocal`,
  `jp.seraf.ktn.syndocal-siw`), the main window maximized, `Responding=True`,
  clean application exit **0**, no processes left running.
- Compiler warnings: **0** in both video configurations, the app tests and
  the native release build. A pre-fix compiler baseline was not remeasured.
  The unchanged frontend emitted one Vite chunk-size warning: App **517.29 kB**
  exceeds **500 kB**. No warning suppression or limit change was added.
- Focused rustfmt and `git diff --check` passed. The optional benchmark was
  not rerun; the table above is unchanged. Actual allocation-failure fallback
  and 4K real-window performance remain unverified.

## SynapseRack comparison: what is and is not known

- No SynapseRack frame times were measured yet.
  - **Approval:** the owner approved installing it (2026-10-15). The official
    free Demo (limited to 4 layers and 2 group layers) is enough for the
    1/2/4-layer comparison.
  - **Downloaded:** `SynapseRack-Windows-Online-v3.3.0-build122-Installer.exe`
    from `dl.synapserack.com`, 1 544 597 166 bytes, SHA-256
    `a21a9767dc9ae7fa8fd4882f17047892932336f54634b4ae00d82e924778558a`.
  - **Pending:** the installer is not code-signed, so the owner runs it.
- To compare (Tier C), on one machine:
  - pin and record the SynapseRack build and licence (the Demo is limited to
    four layers);
  - load this media pack;
  - build 1, 2, 4 and 8 layers at 50 % Add/Screen/Multiply;
  - capture presented frame times for both applications with PresentMon over
    the same 60 s.
- Until then, the honest statement is about Syndocal's own budget, after
  the follow-ups above:
  - **1080p with up to 4 layers:** every codec holds 60 fps on this machine.
  - **6 layers:** within 1–3 frames of 60 fps.
  - **8 layers:** does not hold 60 fps. The look-ahead decodes compete with
    the frame being rendered.
  - **4K Display output:** the headless benchmark in follow-up 4 measures
    GPU composition without decoding. It suggests more budget for decoding
    4K frames; real-window frame times including decoding remain unmeasured.
