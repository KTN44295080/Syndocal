# Syndocal performance envelope — 2026-10-20

## Outcome and performance budget

This campaign measures the product sources at `c204074d3e2d8b2c54134cc295df6ea3a37ae2f1` on `task20-perf`, incorporating `main` at `7fe67b00`. The final documentation commit does not change those product sources. The Display lease boundary fix was reviewed and committed as `82257bd6`; the cache, ISF and operator-readout changes were retained from `4e50f0be` and `a2189fee`.

The after build completed all **63 supported workloads: 22 passed the declared envelope and 41 missed it**. Six planned Display + NDI workloads were unsupported by the required default build. The before comparison contains 16 attempted workloads: two passes, thirteen misses, and one native-backend startup failure with no established measurement. No supported after case is missing.

| Phase | Planned cases | Complete measurements | Partial measurements | Workload failed | Unsupported | Envelope pass | Envelope miss |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Before counterfactual | 16 | 10 | 5 | 1 | 0 | 2 | 13 |
| After | 69 | 47 | 16 | 0 | 6 | 22 | 41 |

These are case counts. The CSV has **98 output rows and 133 columns**, because Display + Spout and planned Display + NDI each have two output rows. All 21 partial measurements across both phases are established misses from another verified failing boundary; none is a pass. The sole supported not-established case is before 16-layer 4K H.264, which exceeded the native-backend startup deadline.

The clearest optimization evidence is the elimination of repeated session opens and evictions in the tested 12- and 16-layer H.264 working sets. Both configurations remain far below 60 fps, and retained private memory rises. The tested FX graph, forced CPU path, group fixtures and 4K output windows also remain outside the declared envelope. Those limitations are documented with observed boundaries and proposed follow-up work.

### Measured 60 fps budget on this configuration

Every passing native client below is 1920 × 1080. Layer counts are the exact tested points, not a guarantee for untested intermediate counts or other media.

| Workload | Passing tested configuration | First tested limitation / qualification |
| --- | --- | --- |
| H.264 1080p30, normal blend, no FX, Display | 1 and 3 layers | Six layers delivered 31.270 fps; higher tested counts also missed. |
| H.264 2160p30 source, normal blend, no FX, 1080p Display | 1 layer | Three layers delivered 58.714 fps and failed the native gate. This is a 4K source with 1080p output. |
| HAP 1080p30, normal blend, no FX, Display | 3, 6, 8, 12 and 16 layers | The single-layer CPU composition case delivered 59.913 fps but failed native work tails. Do not infer a monotonic layer limit. |
| DXV 1080p30, normal blend, no FX, Display | 1, 3, 6 and 8 layers | Twelve layers delivered 57.605 fps; sixteen delivered 46.335 fps. |
| H.264 1080p30, normal blend, no FX, Display + local Spout | 1 layer | Both outputs passed. At three layers, Display/Spout delivered 57.834/57.681 fps and the case missed. |
| Three H.264 1080p30 layers, no FX, 1080p Display | Normal, Add, Multiply, Screen, Overlay, Subtract, Difference, Lighten, Darken and Soft light | All ten tested modes passed. Normal is the same base case listed above; nine modes add distinct passing cases. |
| 4K Display client; enabled one- or three-stage FX; forced CPU; tested groups | No passing tested configuration | Their individual measurements appear below; changing layer count alone does not recover all of these paths. |
| Display + NDI | No measurement established | NDI was `NotBuilt`; no NDI sender was enabled. |

These are **short 60 fps screening results for the exact listed workloads**. Each pass requires complete measurement, successful cadence on every expected output, and the completed native Display work gate. The sources contain 30 images per second; a 60 fps submission rate can repeat source images. The results do not establish a soak-tested capacity, physical 4K scanout, or receiver delivery.

Operator guidance follows the measured configuration. Check actual output FPS and missed intervals alongside `Prepare p95 / budget`; Program/Preview monitor FPS describes a separate path. Recheck after changing source codec, source/output size, output count, FX graph, blend mode or group structure. The layer count alone is insufficient: the tested single-layer HAP path uses CPU composition, while the tested multilayer HAP cases use GPU composition buffers.

The complete numerical record is in [measurements.csv](performance/task20-2026-10-20/measurements.csv). [Detailed tables](performance/task20-2026-10-20/detailed-tables.md) include per-output cadence and preparation p50/p95/p99, counter deltas, decoder demand, native work, CPU/GPU/memory, monitor FPS, coverage and missing evidence. [Per-case findings](performance/task20-2026-10-20/bottlenecks.md) associate every measured miss with its first observed failing boundary and a focused next investigation.

## Machine, build and fixtures

| Item | Recorded configuration |
| --- | --- |
| Machine | POOP; AMD Ryzen AI MAX+ 395; 16 cores / 32 logical processors |
| Graphics | Radeon 8060S; driver `32.0.31032.1003`; Parsec Virtual Display Adapter `0.45.0.0` also installed |
| Operating system | Windows 11 Home, `10.0.26200`, build `26200` |
| OS-visible memory | 59,751,862,272 bytes, approximately 55.65 GiB |
| Enumerated primary display | One 2560 × 1600 display at 180 Hz; scale 2 |
| QA Display client | 1920 × 1080, or an oversized 3840 × 2160 window on that primary monitor |
| Main UI viewport | 1280 × 752 logical pixels for native screenshot checks |
| Native build | Default `build-nobundle.cmd`; `CARGO_TARGET_DIR` unset |
| Available local transports | Display and Spout; Spout2 SDK 2.007.017, CPU-pixel DirectX 11 path |
| Unsupported matrix transport | NDI `NotBuilt`; six planned NDI cases remain explicitly unsupported |
| Other platform feature | Syphon is macOS-only; no Windows Syphon performance claim |
| Harness runtime | Node 22.22.1; Python 3.12; PresentMon 2.6.0 |
| Media tool | FFmpeg 8.1.2, `n8.1.2-44-g7c533d0f86-20260818` |

The display geometry establishes a 4K **client size**, not physical 4K scanout. A post-sample hardware capture recorded the Performance power plan; its state was not continuously recorded during the campaign. Display and local QA Spout are the only enabled test outputs; lighting and real network destinations are excluded. Default release support is not evidence of NDI performance.

| Fixture media | Source format | Encoded bytes | Duration / rate |
| --- | --- | ---: | --- |
| `h264_1080p30.mp4` | H.264, 1920 × 1080, yuv420p | 17,716,533 | 12 s / 30 fps |
| `h264_2160p30.mp4` | H.264, 3840 × 2160, yuv420p | 58,560,899 | 12 s / 30 fps |
| `hap_1080p30.mov` | HAP, 1920 × 1080, rgb0 | 35,146,526 | 12 s / 30 fps |
| `dxv_1080p30.mov` | DXV, 1920 × 1080, rgba | 51,005,989 | 12 s / 30 fps |

Full media, executable, fixture and harness hashes are in [provenance.json](performance/task20-2026-10-20/provenance.json). The H.264/HAP files were retained in the local VJ benchmark media directory; DXV was retained under the earlier `target/qa/task20/native` QA directory. No media was copied from a user show for publication.

The matrix comprises 24 source-format/layer combinations (four codec/resolution configurations × six layer counts), 18 output variants including six unsupported NDI cases, 12 FX configurations, nine additional blend modes, three forced CPU cases and three group cases. Layer counts are 1, 3, 6, 8, 12 and 16 where shown. Blend comparisons use three layers; CPU and group variants use 3, 8 and 16 layers. The group fixture uses opacity 0.9 and Screen blending. A nonzero 0.001° rotation deliberately fails the exact-zero GPU eligibility condition in the CPU fixtures; native diagnostics confirm that no GPU composition layers remain.

FX fixtures use an enabled embedded ISF graph with one or three stages per layer, with a simple RGB × 0.99 operation. Three stages means the root plus two stack children. The harness validates the loaded graph, layer identities, play state, real decode success, output advancement and native render-error state. This characterizes that graph's pipeline cost; arbitrary shaders can have different costs.

The copied QA fixtures initially contained duplicate clip IDs. The QA-only repair assigns `layer.id * 1000 + index1` and remaps references. All 69 copied fixtures were checked for uniqueness; the earlier copies and hashes were archived. User shows and the original starting fixture were preserved. Invalid early fixture probes are excluded from the performance tables.

## Measurement boundaries and acceptance

Each case uses a 10-second harness warmup and a nominal 20-second observation. Health and native-window counters are queried approximately once per second. The native Display gate independently samples 600 work frames after its own 60-frame warmup; this window can finish before the harness observation ends.

| Gate | Predeclared requirements |
| --- | --- |
| Delivered cadence, each expected output | At least 59.4 fps; missed fraction at most 1%; rolling p99 interval at most 33.333 ms |
| Native Display work | Completed 600-frame validation; average at most 16,666 µs; maximum at most 33,333 µs; work-deadline misses at most 5% |
| Measurement completeness | Correct workload and executable, actual output advancement, process/resource evidence and sufficient exact-process presentation evidence |

`missed fraction = Δmissed / (Δdelivered + Δmissed)`. It uses the output telemetry's missed-interval estimate. Native work-deadline misses are a separate counter and denominator. Passing cadence alone does not satisfy the envelope when native work fails or is still sampling.

| Measurement | Meaning and limitation |
| --- | --- |
| Delivered FPS | `ΔdeliveredFrames / ΔmonotonicSeconds`, counting completed Display present returns or accepted sender submissions. This does not count unique source images, physical scanout or received frames. |
| Cadence p50/p95/p99 | Final rolling output-health history, at most 599 intervals. `sampleCount` already counts intervals. Slow cases can retain earlier warmup observations, so these are not exact distributions of only the 20-second observation. |
| Preparation p50/p95/p99 | Up to 600 recent successful preparations, including decode. Display excludes final GPU composition, present and pacing. Sender preparation has its own boundary, including applicable readback but excluding SDK send. |
| Native work per frame | An approximate observation mean reconstructed from differences of rounded lifetime average × frame count. Includes preparation, final composition/present and possible vsync waiting; excludes pacing. Its window differs from the frozen native gate and rolling health quantiles. Unstarted warmup counters remain unavailable. |
| Decoder demand mean | `Δdecode_wall_us / Δdecode_call_count / 1000`, per synchronous demand call or batch. Includes waits, excludes background lookahead, and is not total decoder CPU time or an isolated per-source-frame decode cost. Raw numerator and denominator are retained. |
| CPU and memory | Native app/descendant process samples; retired CPU is retained by PID plus start time. CPU is normalized to 32 logical processors. Private bytes and summed working sets are separate; summed working sets can double-count shared pages. |
| GPU | Sums of raw matching engine instances, with provider/category coverage and instance counts. AMD `Video Codec Engine` is reported separately; sums can exceed 100% and do not represent device-wide utilization. The unobserved decode category is unavailable, not zero. |
| PresentMon | Exact native PID/application and one swapchain; present-call timing, not guaranteed display events. At least ten rows are required for complete statistics. Missing files, empty traces and sparse traces retain distinct evidence states. |
| Program/Preview FPS | Distinct image-object paints through the monitor Canvas path. It measures monitor updates independently of native output, and does not prove unique decoded content. |

Health timestamps are taken **before** awaiting the health query; query-completion latency was not recorded. Endpoint latency differences can bias the counter-derived rate, particularly during IPC stalls. PresentMon and native intervals provide separate corroboration. Treat small differences around 0.1 fps as approximate, not precise speed improvements. Do not subtract preparation p95 from native or decoder means to estimate a component: the distributions, windows and denominators differ.

The process sampler's effective cadence depends on CIM cost; each case's actual sample count and span are recorded in the CSV. It requires at least three samples and a span of at least 60% of the planned observation. The after-run guard waits for host `cargo`, `rustc`, `link` and `cl` processes before each case and rejects sampled overlap. Standalone test executables and other GPU workloads are not comprehensively monitored. Before runs used manual startup/between-case checks, without per-sample host-build monitoring. The earlier before sampler retained discovered descendant PIDs between iterations; after sampling rebuilds current ancestry. These differences limit causal and resource comparisons.

Missing or sparse presentation evidence can retain a **partial measurement / envelope miss** when another verified boundary independently fails. It cannot establish an envelope pass. An absent native-backend startup or a failed workload remains not established. No missing measurement is replaced with zero. The report uses the same final aggregation logic for both phases, with unchanged acceptance thresholds.

## Full measured matrix and matched before/after results

### After: source codec sweep

Cells show delivered output FPS / envelope status. Display is 1080p with normal blend and no FX unless stated. FPS has three decimal places; missing metrics stay unavailable. Status tokens come from the CSV. A workload_failed or partial row is never promoted to pass; diagnostic measurements can remain visible beside its restricted status.

| Source (30 fps) | 1L | 3L | 6L | 8L | 12L | 16L |
| --- | --- | --- | --- | --- | --- | --- |
| H.264 1080p | 59.911 / pass | 59.938 / pass | 31.270 / miss | 27.933 / miss | 2.761 / miss | 0.365 / miss [partial] |
| H.264 4K | 59.863 / pass | 58.714 / miss | 0.829 / miss [partial] | 0.615 / miss [partial] | 0.154 / miss [partial] | 0.204 / miss [partial] |
| HAP 1080p | 59.913 / miss | 60.002 / pass | 59.998 / pass | 59.937 / pass | 59.924 / pass | 60.020 / pass |
| DXV 1080p | 60.006 / pass | 59.997 / pass | 59.954 / pass | 59.983 / pass | 57.605 / miss | 46.335 / miss |

### After: output and FX variants

All variants use H.264 1080p sources and normal blend. The two Display + Spout rows are separate outputs from the same cases. NDI rows retain the build's unsupported classification.

| Variant / output | 1L | 3L | 6L | 8L | 12L | 16L |
| --- | --- | --- | --- | --- | --- | --- |
| Display 4K | 43.939 / miss | 23.641 / miss | 6.344 / miss | 3.860 / miss | 0.965 / miss [partial] | 0.356 / miss [partial] |
| Display + Spout: Display | 59.989 / pass | 57.834 / miss | 18.890 / miss | 4.260 / miss | 0.308 / miss [partial] | 0.203 / miss [partial] |
| Display + Spout: Spout | 59.938 / pass | 57.681 / miss | 18.634 / miss | 4.106 / miss | 0.308 / miss [partial] | 0.203 / miss [partial] |
| Display, 1 FX/layer | 50.934 / miss | 11.032 / miss | 4.366 / miss | 2.628 / miss | 0.718 / miss [partial] | 0.307 / miss [partial] |
| Display, 3 FX/layer | 20.678 / miss | 5.283 / miss | 2.424 / miss | 1.180 / miss [partial] | 0.563 / miss [partial] | 0.359 / miss [partial] |
| Display + NDI: Display | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] |
| Display + NDI: NDI | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] | unavailable / not_established [unsupported; unavailable] |

### After: blend modes, CPU requests and groups

H.264 1080p sources; Display 1080p. Native and cadence columns retain the individual CSV gate statuses; neither overrides the envelope status.

| Case | Layers | Output FPS | Prepare p95 (ms) | Native work | Cadence | Envelope |
| --- | --- | --- | --- | --- | --- | --- |
| Blend: Normal | 3 | 59.938 | 2.952 | pass | pass | pass |
| Blend: Add | 3 | 60.004 | 0.439 | pass | pass | pass |
| Blend: Multiply | 3 | 60.016 | 0.447 | pass | pass | pass |
| Blend: Screen | 3 | 60.016 | 0.487 | pass | pass | pass |
| Blend: Overlay | 3 | 59.983 | 0.453 | pass | pass | pass |
| Blend: Subtract | 3 | 59.984 | 0.456 | pass | pass | pass |
| Blend: Difference | 3 | 59.958 | 0.484 | pass | pass | pass |
| Blend: Lighten | 3 | 60.004 | 0.471 | pass | pass | pass |
| Blend: Darken | 3 | 60.005 | 0.466 | pass | pass | pass |
| Blend: Soft light | 3 | 59.968 | 0.474 | pass | pass | pass |
| CPU requested | 3 | 11.000 | 243.596 | not_established | fail | miss |
| CPU requested | 8 | 2.486 | 1883.929 | not_established | fail | miss |
| CPU requested | 16 | 0.552 | 4183.023 | not_established | fail | miss [partial] |
| Groups | 3 | 57.510 | 14.323 | fail | fail | miss |
| Groups | 8 | 19.953 | 168.011 | not_established | fail | miss |
| Groups | 16 | 0.407 | 4414.881 | not_established | fail | miss [partial] |

### Exact before → after pairs

All 16 before case tags / 17 output rows are included. Pairs match the exact case tag, output kind and measured output ID; an ID-less planned row only records unavailability. Display and Spout are separate rows. Arrows mean the before value followed by the after value, not an arithmetic difference. Δmissed, Δopens and Δevictions are each run's observation-period counter deltas. Decode demand is mean wall time per demand call, not GPU decode utilization.

| Case | Output | FPS: before → after | Δmissed: before → after | Prepare p95 (ms): before → after | Decode demand mean (ms/call): before → after | Δopens/Δevictions: before → after | Envelope: before → after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| H.264 1080p, 1L, Display | Display #1 | 60.035 → 59.911 | 0 → 1 | 4.454 → 3.741 | 0.163 → 0.164 | 0/0 → 0/0 | pass → pass |
| H.264 4K, 1L, Display | Display #1 | 59.730 → 59.863 | 5 → 3 | 9.856 → 8.196 | 0.248 → 0.194 | 0/0 → 0/0 | miss → pass |
| H.264 1080p, 3L, Display | Display #1 | 59.904 → 59.938 | 2 → 1 | 1.830 → 2.952 | 0.276 → 0.257 | 0/0 → 0/0 | pass → pass |
| H.264 1080p, 6L, Display | Display #1 | 31.735 → 31.270 | 561 → 561 | 113.198 → 100.755 | 10.285 → 6.620 | 0/0 → 0/0 | miss → miss |
| H.264 1080p, 8L, Display | Display #1 | 12.851 → 27.933 | 912 → 624 | 244.873 → 113.212 | 42.858 → 7.001 | 0/0 → 0/0 | miss → miss |
| H.264 4K, 8L, Display | Display #1 | 0.152 → 0.615 | 1197 → 901 | 9755.260 → 4505.291 | 6609.333 → 1225.160 | 0/0 → 0/0 | miss [partial] → miss [partial] |
| H.264 1080p, 12L, Display | Display #1 | 0.306 → 2.761 | 1350 → 1140 | 5105.741 → 2132.619 | 3725.401 → 299.313 | 72/72 → 0/0 | miss [partial] → miss |
| H.264 1080p, 16L, Display | Display #1 | 0.150 → 0.365 | 1008 → 967 | 7347.754 → 4188.358 | 5567.554 → 2267.443 | 48/48 → 0/0 | miss [partial] → miss [partial] |
| H.264 4K, 16L, Display | Display | unavailable → 0.204 | unavailable → 1347 | unavailable → 9713.340 | unavailable → 5495.351 | unavailable/unavailable → 0/0 | not_established [workload_failed] → miss [partial] |
| H.264 1080p, 3L, Display 4K | Display #1 | 14.330 → 23.641 | 893 → 701 | 110.920 → 78.916 | 33.980 → 8.666 | 0/0 → 0/0 | miss → miss |
| H.264 1080p, 3L, Display + Spout | Display #1 | 59.896 → 57.834 | 2 → 42 | 5.391 → 1.320 | 0.357 → 0.644 | 0/0 → 0/0 | miss → miss |
| H.264 1080p, 3L, Display + Spout | Spout #2 | 59.946 → 57.681 | 1 → 46 | 11.386 → 9.239 | unavailable → unavailable | unavailable/unavailable → unavailable/unavailable | miss → miss |
| H.264 1080p, 1L, 3 FX/layer | Display #1 | 17.211 → 20.678 | 829 → 761 | 68.130 → 56.015 | 8.001 → 4.708 | 0/0 → 0/0 | miss → miss |
| H.264 1080p, 3L, 3 FX/layer | Display #1 | 4.761 → 5.283 | 1072 → 1046 | 345.731 → 416.199 | 50.696 → 57.321 | 0/0 → 0/0 | miss → miss |
| H.264 1080p, 8L, 3 FX/layer | Display #1 | 0.573 → 1.180 | 1211 → 1041 | 3933.381 → 2148.322 | 1338.168 → 454.711 | 0/0 → 0/0 | miss [partial] → miss [partial] |
| H.264 1080p, 8L, CPU requested | Display #1 | 1.800 → 2.486 | 1192 → 1140 | 2725.845 → 1883.929 | 390.282 → 253.060 | 0/0 → 0/0 | miss [partial] → miss |
| H.264 1080p, 8L, Groups | Display #1 | 18.704 → 19.953 | 814 → 721 | 324.846 → 168.011 | 15.071 → 11.908 | 0/0 → 0/0 | miss → miss |

## Fixes and interpretation

The before executable is a controlled counterfactual on the same merged product tree: cache capacity eight and an RGBA clone in the ISF input path. The after executable restores capacity sixteen and borrows that input. Both use eight decode workers and the same product instrumentation. This is not a historical-release comparison. The two variants were built with the official wrapper and passed the native launch gate; the temporary source changes were restored byte-for-byte before the after build.

| Change | Implementation and evidence |
| --- | --- |
| Bound the libav working set at 16 and retain parallel admission | `crates/video/src/libav_decoder.rs:47`, worker pool at `:195–204`, admission at `:1136`, session decision at `:1170`; regression cases in `crates/video/src/libav_parallel_decode_tests.rs:449`, `:472`, `:499`. The worker limit remains eight. |
| Borrow existing RGBA input for ISF | `crates/video/src/isf_pass_runtime.rs:190`; removes one owned clone where the RGBA input can be borrowed. The FX comparison below reports differences between the combined binary variants for the tested graph; it does not isolate a repeatable effect size for borrowing alone. |
| Report preparation budget and current output health | `app/src-tauri/src/live_video_output_health.rs:103`, `:174`, `:222`; Display integration in `app/src-tauri/src/main.rs:85278`; Spout in `app/src-tauri/src/spout_transport.rs:1726`; NDI in `app/src-tauri/src/ndi_transport.rs:1297`. |
| Prevent stale or reused preparation samples | `app/src/createLiveVideoOutputHealthController.ts`, retained from `a2189fee`; focused controller and performance-budget checks pass. |
| Gate physical Display callbacks on successful authorization receipts | `app/src-tauri/src/native_display_lease_boundary.rs`, called from `app/src-tauri/src/main.rs:82908`; reviewed inherited work committed as `82257bd6`. Rejected operations must not reach the native callback. |

The after cases have 23 cadence passes and 40 cadence failures; native work has 23 passes, ten failures and 30 unfinished validation windows. Their common passing set has 22 cases. HAP1 passes cadence but fails native work; three-layer Display + Spout passes native work but fails cadence. An unfinished native window cannot establish a pass.

### Cache retention improves; the high-layer H.264 envelope still misses

The controlled counterfactual keeps the merged source and instrumentation fixed while changing capacity 8 → 16 and RGBA cloning → borrowing; both binaries retain eight decode workers. The no-FX 12-/16-layer H.264 1080p pairs exercise the capacity change without the ISF borrowing path:

| Layers | Submitted fps before → after | Δopens / Δevictions before → after | Private-byte peak GiB before → after |
| --- | --- | --- | --- |
| 12 | 0.305713 → 2.760827 | 72 / 72 → 0 / 0 | 4.6443 → 5.4273 |
| 16 | 0.150172 → 0.364513 | 48 / 48 → 0 / 0 | 4.9463 → 6.6870 |

The after decoder snapshots retain 12/12 and 16/16 hardware/total sessions, versus eight/eight before. Eviction elimination is directly measured. Source review also identifies the parallel-admission condition: retained owners plus new owners must fit capacity, with the other eligibility conditions satisfied. Capacity therefore affects both retention and batch eligibility while worker concurrency stays bounded at eight.

Remaining preparation p95 is **2,132.6192 ms at 12 layers** and **4,188.3582 ms at 16 layers**; synchronous decoder-demand means are **299.312685 ms** and **2,267.443429 ms**. The remaining failure is already observable within preparation and decoder-demand waiting/execution. These numbers do not identify hardware decode execution, lookahead starvation, transfer work, or reset feedback as the isolated cause. Increased private-byte peaks document the retention tradeoff; they do not prove resident-memory pressure. Evidence: the matched `h264-1080p30-{12,16}l-display-0fx-normal` cases.

### The no-FX controls constrain causal claims

Normal H.264 1080p at one and three layers remains near 60 fps. Six layers changes little: **31.734542 → 31.269977 fps**. Eight-layer 1080p instead changes **12.851100 → 27.932947 fps**, and eight-layer 4K source changes **0.152206 → 0.615351 fps**. Both eight-layer comparisons report zero opens/evictions in both observation intervals and unchanged observed composition paths.

Those approximately 2.17× and 4.04× single-pair differences cannot automatically be credited to capacity 16 or borrowing: the active workloads already fit eight retained owners and do not use ISF. Actual admission/history and repeated measurements are needed to explain the change. The three-layer 4K-output no-FX control also changes substantially, **14.329832 → 23.640514 fps**, while remaining far below target. Background-build monitoring was stronger during after runs, and the before/after response and rolling-history windows differ. The retention conclusion rests on the eviction counters; FPS effect-size attribution needs that additional control.

### Effects: exact pairs improve cadence unevenly and all miss the envelope

The matched three-effect H.264 1080p results are:

| Layers | Submitted fps before → after | Prepare p95 ms before → after |
| --- | --- | --- |
| 1 | 17.210501 → 20.678472 | 68.1298 → 56.0151 |
| 3 | 4.761498 → 5.282777 | 345.7310 → 416.1994 |
| 8 | 0.573318 → 1.180019 | 3,933.3814 → 2,148.3216 |

All three remain cadence misses. The three-layer case has a higher preparation p95 despite better aggregate FPS, so the evidence does not support a uniform tail improvement. These ≤8-layer pairs have zero opens/evictions in both intervals. A separate borrowing-only comparison with matched admission and actual execution would be needed to estimate its individual effect. Short histories, reset/request trajectories, and run conditions still limit the measured effect size. Eight-layer preparation has only 23 before and 35 after samples; its percentiles are descriptive.

All 12 after FX cases miss. Even one layer with one effect reaches only **50.934489 fps**, with preparation p95 **20.6657 ms**. At three layers, one effect reaches **11.032462 fps** and three effects **5.282777 fps**. Multi-layer FX cases report CPU composition with no retained GPU composition layers. That label describes composition eligibility; it does not locate every shader operation on the CPU. The earliest observed limits are preparation, and in heavier cases synchronous demand latency within preparation. A per-effect cost cannot be obtained by subtracting these quantiles. Evidence: the `h264-1080p30-…-display-{1,3}fx-normal` cases.

### 4K source and 4K output are different tested workloads

With **4K H.264 source and a 1080p Display client**, one layer passes at **59.862550 fps**. Three layers misses at **58.713924 fps**, preparation p95 **23.5989 ms**, and native maximum **38.934 ms**. At six/eight layers output falls below one fps. The canonical 12-/16-layer reruns establish severe cadence misses at **0.153916 / 0.203882 fps**, with preparation p95 **8,315.6084 / 9,713.3402 ms** and demand means **6,155.591333 / 5,495.351250 ms**. Their five/six preparation samples and one/two PresentMon rows cannot establish a stable tail or a meaningful ranking between those layer counts. Their native work windows remain unstarted. Evidence: `h264-2160p30-…-display-0fx-normal`.

With **1080p source and a 4K Display client**, all six tested layer counts miss. One layer reaches **43.939327 fps**, preparation p95 **23.9483 ms**; three layers reaches **23.640514 fps**, preparation p95 **78.9165 ms**. Six/eight layers reach **6.344281 / 3.860333 fps**, and 12/16 layers reach **0.965405 / 0.355659 fps**. The preparation boundary already exceeds budget, before the separately excluded final Display composition/present. The observations do not isolate scaling, transfers, CPU work, or final GPU cost. The 3840×2160 client is an oversized window on the available display, not evidence of physical 4K scanout. Evidence: `h264-1080p30-…-display4k-0fx-normal`.

### Display+Spout requires two output verdicts

One layer passes with Display **59.989203 fps** and Spout **59.937930 fps**. At three layers, both miss: Display **57.834489 fps**, Spout **57.681488 fps**, with missed fractions **3.5714% / 3.9082%**. Preparation p95 remains **1.3197 / 9.2392 ms**, and the native Display validation passes with maximum **30.075 ms**. PresentMon corroborates Display cadence at **57.966059 fps**. Here the first established failing boundary is delivered cadence; the existing preparation/native measurements do not isolate SDK send, pacing, scheduling, or cross-output contention.

At six/eight layers, Display/Spout rates fall to **18.890204 / 18.634239** and **4.260474 / 4.106481 fps**. At 12 and 16 layers, both output counters report **0.308274** and **0.202906 fps**, respectively, with multi-second preparation histories. The simultaneous output workload is more demanding in these cases, but its cost cannot be assigned entirely to sender transport. Spout health measures accepted submissions, not receiver delivery. Evidence: `h264-1080p30-…-spout-0fx-normal`, retaining both output rows.

### Blends, forced CPU composition, and groups

At three H.264 1080p layers with no FX, **all nine non-normal blend variants pass**, as does Normal. Add, Multiply, Screen, Overlay, Subtract, Difference, Lighten, Darken, and SoftLight show observed GPU composition, rates **59.958072–60.016478 fps**, and preparation p95 **0.4390–0.4873 ms**. This supports those tested blend configurations; it does not establish their higher-layer limits or intrinsic speed relative to Normal.

Fixtures forcing CPU composition at three/eight/16 layers miss at **10.999600 / 2.486072 / 0.551833 fps**, with preparation p95 **243.5961 / 1,883.9291 / 4,183.0231 ms**. Actual composition diagnostics confirm the CPU path. These fixtures change eligibility; they associate the missed envelope with that executed workload without timing an isolated CPU routine.

Grouped cases report **CPU group isolation with GPU outer composition**. Three layers reaches **57.509883 fps**; preparation p95/p99 is **14.3226 / 14.9637 ms**, but native work misses **6.5%** of deadlines, exceeding the 5% policy while mean and maximum remain within their separate limits. At eight/16 layers, output falls to **19.953340 / 0.407050 fps** and preparation p95 grows to **168.0107 / 4,414.8806 ms**. The three-layer result calls for matched native-work/cadence timing; the heavier results already expose preparation stalls. Evidence: the `…-normal-cpu` and `…-display-groups` cases.

### HAP and DXV show why one layer count cannot summarize the product

HAP at one layer submits **59.913044 fps** and passes cadence, but native work fails at **36.712 ms maximum** and **6.3333% work misses**. Its single-layer CPU composition path is deliberate. HAP at three/six/eight/12/16 layers uses observed GPU composition and passes both gates, including **60.019769 fps at 16 layers**. This nonmonotonic result crosses composition paths and does not establish a codec-only cause.

DXV passes at one/three/six/eight layers. At 12 layers it reaches **57.604622 fps**, with preparation p95/p99 **12.0885 / 24.8614 ms**, native maximum **50.717 ms**, and work misses **51.1667%**. At 16 layers it reaches **46.335025 fps**, with preparation p95/p99 **2.2631 / 32.5792 ms**, native mean/maximum **17.486 / 75.204 ms**, and work misses **62.3333%**. PresentMon agrees closely at **57.541577 / 46.393220 fps**. Preparation p95 alone misses relevant tails; the native window does not isolate final GPU execution. DXV's native decoder snapshots report zero hardware sessions and 12/16 total sessions, while HAP reports zero libav sessions; neither count describes all process-wide decoding or GPU activity. Evidence: `hap-…-display-0fx-normal` and `dxv-…-display-0fx-normal`.

### Focused next investigations

Prioritize three diagnostic tracks. First, use the smallest failing H.264 preparation cases to correlate synchronous demand waits, lookahead readiness, conversion/transfer work, and bounded reset-reason/target-gap summaries; resets are not opens or evictions, and their counts alone do not justify changing seek semantics. Second, repeat the matched ≤8-layer FX and no-FX controls under comparable load to estimate the borrowing effect and explain unexpected control changes. Third, measure matched final-work/present/send/pacing boundaries for HAP1, DXV12, grouped three-layer output, and Display+Spout at three layers. Those cases distinguish preparation tails, native-work failures, and delivery failures more clearly than a single aggregate FPS or resource percentage.

The strongest measured boundary is identified for each failing case in the [bottleneck appendix](performance/task20-2026-10-20/bottlenecks.md). Preparation can include decoding, conversion, transfer, ISF and waits. A preparation tail alone does not isolate those components. Likewise, a native maximum above the gate does not isolate GPU saturation, present/vsync, scheduling or locking. The next investigation should add matched component timings around the smallest failing workload rather than infer component costs by subtracting unrelated quantiles.

## Operator readout and native visual checks

The readout places actual output FPS and recent missed intervals above `Prepare p95 / budget` in milliseconds. One frame is approximately 16.7 ms at the 60 fps target. The preparation budget needs at least 15 fresh samples; a last-frame age above one second is displayed as stalled. Warnings distinguish missed frames, preparation over budget and stale output. Low preparation time does not guarantee timely final delivery.

Implementation: `app/src/components/LiveVideoOutputHealthReadout.tsx:44`, `app/src/liveVideoPerformanceBudget.ts:15`, `:22`, `app/src/styles.css:5813–5829`, and JA/EN entries in `app/src/uiLocalization.ts:126`, `:130`. Styles use theme tokens and 14px text.

Nine screenshots were visually inspected and bound to the exact hashes in [reviewed-images.json](performance/task20-2026-10-20/reviewed-images.json). Their PNG bytes were copied unchanged; chunk checks found no text/EXIF metadata. Native clients are 1920 × 1080; main-window screenshots are 2560 × 1504 pixels for the 1280 × 752 logical viewport. The recorded readout leaves use 14px text and fit within their recorded positive-sized readout bounds.

| Native workload | English readout | Japanese readout | Native Display image |
| --- | --- | --- | --- |
| Three layers, no FX, Display | [EN image](performance/task20-2026-10-20/images/after-h264-1080p30-3l-display-0fx-normal-en.png) | [JA image](performance/task20-2026-10-20/images/after-h264-1080p30-3l-display-0fx-normal-ja.png) | [QA frame 101](performance/task20-2026-10-20/images/after-h264-1080p30-3l-display-0fx-normal-native-display.png) |
| Three layers, no FX, Display + Spout | [EN image](performance/task20-2026-10-20/images/after-h264-1080p30-3l-spout-0fx-normal-en.png) | [JA image](performance/task20-2026-10-20/images/after-h264-1080p30-3l-spout-0fx-normal-ja.png) | [QA frame 123](performance/task20-2026-10-20/images/after-h264-1080p30-3l-spout-0fx-normal-native-display.png) |
| Three layers, three FX stages per layer | [EN image](performance/task20-2026-10-20/images/after-h264-1080p30-3l-display-3fx-normal-en.png) | [JA image](performance/task20-2026-10-20/images/after-h264-1080p30-3l-display-3fx-normal-ja.png) | [QA frame 82](performance/task20-2026-10-20/images/after-h264-1080p30-3l-display-3fx-normal-native-display.png) |

The heavy FX captures show **5.3 output fps**, `Prepare p95 412.1 / 16.7 ms`, and the amber `Frames missed` / `フレーム欠落` warning. The Program monitor shows 0 fps with output protection while the native Display still contains the QA media. The two-output capture shows separate Display and Spout preparation values, approximately 1.4 and 9.2–9.4 ms, in both locales.

The post-observation Display + Spout readouts show approximately 59.9 fps. They are later rolling UI observations, with recent-miss windows distinct from the benchmark interval; they do not replace that case's measured 57.834/57.681 fps or its envelope miss; its native work gate passed.

These screenshots were taken after timed observations. Native images were captured from the exact app-owned Display client on the primary monitor and checked for nonblack QA media. Screenshot checks do not alter the timed measurement interval or establish physical scanout. See [ui-validation.json](performance/task20-2026-10-20/ui-validation.json) for selected bounds, labels and image hashes.

## Validation and merge accounting

| Check | Result |
| --- | --- |
| `cargo check --workspace --all-targets` | Passed with zero warnings |
| Full `cargo test -p syndocal --bin syndocal` | 2,205 passed; one failed; 23 ignored; 2,229 total |
| Exact isolated rerun of the failing test | One passed, single thread; 2,228 filtered |
| Lease boundary tests | Two passed |
| Combined NDI smoke test after merge | One passed |
| Relevant video libav filter | 34 passed; six ignored; 265 filtered |
| Relevant ISF filter | 38 passed; zero ignored; 267 filtered |
| After `build-nobundle.cmd` | Passed in 7 min 54 sec |
| After native gate | Visible titled Syndocal window, responsive; `CloseMainWindow` true; exit 0; zero remaining app processes |
| TypeScript / frontend build / localization / Perform layout | Passed |
| Monitor controller / performance budget | Passed |
| Theme browser | Eight themes, 96 type-role checks, 96 contrast checks, 11 interactions; zero errors, warnings or failures; eight Japanese theme names checked |
| Broad viewport baseline comparison | Five viewports; current `c204074d` and merged-main baseline `7fe67b00` both passed; zero new failures |

The full app test run failed at `project_backup_listing::tests::project_backup_listing_missing_directory_pagination_exact_bytes_and_no_allocation`, reported line 95, expected `1` and observed `0`. The exact isolated single-thread rerun passed. The full run is not an unconditional pass, and an isolated pass does not prove the cause of the first failure.

The frontend build retains Vite's existing large-chunk notice. Initial browser cold mounts exceeded an eight-second deadline; warm runs passed without weakening assertions. The baseline archive needed the exact `7fe67b00` crates sources before its final pass. The native build followed `qa/WINDOWS_NATIVE_BUILD.md`; `CARGO_TARGET_DIR` remained unset.

Pinned accounting after the merge equals merged-main values plus this branch's zero delta: Tauri 587, Engine 285, Remote 116, MIDI/OSC/DMX 239, Frontend 507, Keyboard 33, and UI literals 347. Supporting values are routing 137, renderer mutation 31, server 28 and raw facade 501. Eighteen inventory-negative, two runtime-order and three sorted-list checks passed. The combined NDI smoke test preserves both the null-decoder black result and the deterministic sample-decoder nonblack result; it does not establish an NDI output benchmark.

Independent source-only review covered the supplied Display lease authorization change and inherited native performance diff: the receipt gate and tests, preparation-health telemetry and affected outputs, synchronous decoder-demand accounting, cache capacity 16 with eight workers, and ISF RGBA borrowing. No blocking correctness issue was identified in that supplied diff and surrounding context. The review did not cover the entire native runtime or unrelated merged-main contributions. Frontend changes were reviewed separately. An independent CSV audit checked exported arithmetic, native gates and PresentMon corroborating aggregates; it did not independently rerun the native executable or reconstruct raw event quantiles. The main-checkout agent performed the native work. Claude's independent native/runtime review remains explicitly requested below.

The final independent CSV audit matched all **85 planned phase/case records** and 98 output rows. It confirmed 24 supported passes, 54 misses and one supported not-established case across both phases; after alone is 22 passes and 41 misses. Every pass has complete evidence, a successful workload, verified PresentMon process identity, sufficient presentation rows, matching fixture hashes, all required output cadence gates and a completed 600-frame native Display pass. No partial case passes. All 63 supported after cases declare sampled build monitoring with no observed compiler overlap.

Recalculated delivered FPS differed by at most 0.000001665 fps from exported frame deltas and elapsed time. Target ratios, missed fractions, decoder demand means and native deadline-miss fractions agreed within 0.0000005. Completed native states agree with their numerical thresholds. For passing Display cases across both phases, counter FPS is 59.862550–60.034603 and PresentMon FPS is 59.888794–60.074695; the median absolute difference is 0.05544 fps and maximum 0.14458 fps. The two clocks/windows remain distinct.

The normalized UTF-8/LF CSV hash is `c465b39b6ad220bed34e5bb4377daf48a6e2646eea355de50cd9c52dac20cb00`. This audit verifies aggregate consistency, not raw event quantiles, independent native execution, physical display events or receiver behavior. [validation.json](performance/task20-2026-10-20/validation.json) links the retained validation logs, final restoration proof and after-release native denial proof.

## Recovery, retained evidence and final state

The final exact after executable rejected a Display-open request made with the insufficient Video-only lease. No Display window opened and output health stayed empty. Successful matrix opens used the required lease sequence, with effective role Video and lighting disabled. The earlier successful before-build denial proof was preserved before this final check.

Canonical after measurements required shutdown recovery in four heavy cases: 12- and 16-layer 4K H.264 sources, 16-layer H.264 1080p with a 4K Display client, and the eight-layer group fixture. The reported failures included output ownership waiting on an in-flight operation, output workers exceeding the 1,000ms retirement deadline, and missing window-retirement acknowledgement. Each recovery stopped only the exact owned app and verified Standby, outputs off and exact state across **two fresh launches**. These are recorded runtime limitations; successful recovery does not fix their cause.

Seven attempts with sampled compiler overlap were archived and excluded. An earlier immediate post-force shutdown check also failed; the exact app was subsequently observed absent, state was verified across two fresh launches, and the stop helper gained a bounded ten-second exit wait. Its original record was preserved. Two later attempts lacked PresentMon files despite a zero tool exit code. Their original failures were archived; the reviewed verifier now distinguishes absent files from empty or sparse captures without permitting a pass. The final 12-/16-layer 4K reruns produced only one/two PresentMon rows and remain partial misses, with no PresentMon FPS or quantiles established.

The original 16-layer one-FX run recorded an opaque lease-renewal object error. The failed workload was excluded and retained, error serialization was improved, and the single diagnostic rerun completed without that error. This does not establish the cause of the earlier failure or a runtime fix. No acceptance threshold or authorization rule was weakened.

At **2026-10-20 09:41:24.587 UTC (18:41:24.587 JST)**, a final clean launch verified **Standby, every live and test-pattern window off, and exact original Recent/localStorage**. The final process check at 09:46:26 UTC found zero owned release processes and zero owned benchmark helpers, with `CARGO_TARGET_DIR` unset. Product sources remained exactly at `c204074d` on `task20-perf`. The original preference snapshot was preserved throughout, and user shows were not overwritten.

The committed evidence uses an explicit field allowlist and preserves numerical/status/hash strings. Absolute machine paths and recognized error paths are normalized in the public CSV. Raw case JSON can contain loaded project state, Recent/localStorage values and error UI; those files remain in the ignored QA directory. The original state snapshot was never overwritten. The manifest identifies retained attempts and input hashes without publishing personal state.

The after executable SHA-256 is `a1e0ae6519bb3558d2b473fc9d68b9a2487ab6ac55ec8e5bbacf1bb375ef869d`; before is `e0257cad4a191953554dee4d57defaa9138646c5915b9fe776b7316b622e9e38`. Reproduction requirements and the archived current harness are described in the [evidence README](performance/task20-2026-10-20/README.md). Exact original aggregates require the retained raw inputs. Current harness hashes do not retrospectively prove earlier script bytes; phase/case hashes and hardening differences are recorded separately.

## Native/runtime handoff for independent review

- Confirm the exact lease receipt gates physical Display open/close callbacks, including denial with an insufficient lease and successful QA open after the required lease/arm sequence.
- Review cache ownership, bounded capacity 16, parallel admission, eight-worker lifetime, release behavior and seek correctness. Opens/evictions and in-place reset counters must remain distinct.
- Review borrowed RGBA lifetime, format conversion and nested ISF execution; the optimization must not change image semantics or alias mutable output storage.
- Review health timing boundaries, bounded sample histories and controller freshness under output replacement, long stalls and disposal.
- Review the combined NDI smoke assertions alongside the preserved Syphon/readout integration. NDI runtime performance remains unsupported in this default build.
- Investigate bounded shutdown under overloaded output preparation: native workers exceeded the 1,000ms retirement deadline in recorded heavy cases. The QA recovery establishes final state, not a fix for that runtime limitation. A proposed follow-up is interruptible/bounded decoder work and cancellation checks across preparation, with an overloaded shutdown regression that preserves lease and output-state correctness.

No push, checkout switch to `main`, or merge into `main` was performed. The result handoff is written after the final QA commit and final restoration checks.
