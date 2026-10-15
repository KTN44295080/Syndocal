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
- Until then, the honest statement is about Syndocal's own budget:
  - **1 layer:** HAP holds 60 fps. H.264, HEVC and ProRes meet it except on
    some frames that decode.
  - **2 or more blended 1080p layers:** do not hold 60 fps on this machine
    even after this change.
  - **Biggest remaining cost:** synchronous decode, then per-tick frame copies.
