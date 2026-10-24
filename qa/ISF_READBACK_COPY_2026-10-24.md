# ISF packed readback copy — 2026-10-24

Base: shared `main` at `693182041ea1d52cd0afa48bc5899bf7f9407b16`.
Owner: Codex. Scope: `isf_pass_runtime.rs`, `isf_stack_workspace_tests.rs`, and this note.

## Behavior

When RGBA row bytes already equal the GPU readback pitch, copy the mapped frame
as one contiguous slice. Keep the existing per-row unpack for padded widths.
The mapped guard still drops before unmap, and feedback ownership is published
only after successful readback. No output, fault, LastValid, or cancellation
policy changes.

## Focused evidence

Evidence directory: `target/qa/isf-readback-copy-2026-10-24/` (ignored local evidence).
Both release/libav Cargo runs initialized MSVC 14.44.35207 and verified its
absolute linker pin and PATH-first resolution in their actual logs.

| Check | Observed result |
| --- | --- |
| `cargo test -p video --features libav --release workspace_tests -- --test-threads=1 --include-ignored --nocapture` | 4 passed, 0 failed, 0 ignored; includes real GPU pixels, aligned/padded widths, resize/reuse and failure cleanup |
| `cargo test -p video --features libav --release isf -- --test-threads=1` | 76 passed, 0 failed, 2 opt-in timing experiments ignored |
| Independent stable-diff review | `review_readback_copy`: no actionable blockers; checked exact mapped size, padded behavior, unmap and feedback order |

The opt-in `mapped_readback_copy_timing_experiment` compares the old row loop
against the new contiguous path on the same mapped 2560×1600 real GPU buffer.
Each round uses old/new/new/old order; four warmup rounds are discarded, leaving
32 samples per path. Pixel equality and deallocation are outside timed sections.

| Allocation + copy only | Old row loop | Contiguous copy |
| --- | ---: | ---: |
| Median | 2.7698 ms | 2.3102 ms |
| p95 | 3.3592 ms | 2.9199 ms |

The observed median is 16.6% lower for this warm copy experiment. This is not
GPU render, map/wait, unmap, composition, native frame time, or an FPS speedup
claim. The test has no wall-time assertion.

## Windows native context

Campaign: `fb2a460b-1788-4bcb-a637-bc6fd1cec9ac`, debugger port 9273.
The isolated target `target/readback-copy-20261024` built successfully with
`pnpm --dir app tauri build --no-bundle --config <campaign>/tauri.qa.conf.json`.
Built exe SHA-256: `76c60a6102820bef8c989d5f80c80bb0719c6bc2a6173a263b988ab7d164d6bb`.
Exactly one responsive window from that executable was launched as PID 46544,
then maximized and verified in Standby before output actions.

The build and tests also include the preserved, unadopted borrowed-input candidate
in `lib.rs` (SHA-256 `6a672620ecc25a06a72fbfe9f64a69091268c4409fa5c0cfd851138701a12430`)
and `isf_borrowed_input_tests.rs`
(`029c77acd5a8a5997734ba5983b1f6f0801fb8ae180967730ada776c33d89ca3`).
Those files belong to Steroids and are excluded from this checkpoint. Their
hashes remained unchanged. This native run cannot isolate either candidate's FPS
effect. The normal checkout executable remains unchanged at
`148e27b77afb2515d668a32e0d9cd7b0b6f10479dc9db54038128b01d1430cf3`;
the pre-existing peer app PID 19836 was observed in Standby and was not stopped.

`perf-h264-1080-3l-3fx-readback-01/evidence.json` records a trace/log-off run
with known silent H264 media, 3 layers × 3 FX stages, 5-second warmup and
60-second capture. Native Arm and local Display confirmation dialogs were
observed and confirmed once; lighting remained disallowed.

| Native observation | Result |
| --- | --- |
| Actual output | 2560×1600 |
| Application delivery counter | 1715 deliveries / 59.6843671 s = 28.7345 fps |
| Render wall time p50 / p95 | 31.3776 / 50.5300 ms; excludes GPU composition/present |
| Validation budget | 600 / 600 misses; **60 fps acceptance failed** |
| Decode failures / lifetime output errors | 0 / 0 |
| Stop wall time | 707.6515 ms |
| After Stop | Ready / Standby; lighting and video disallowed; live/test windows closed; active routes and health empty |
| Normal close | QA PID and all 6 recorded WebView children absent; port 9273 listener absent; original app PID 19836 remains responsive |

The observation harness's `passed: true` means its delivery, error and bounded
Stop assertions passed. It does **not** override the failed frame-budget result.
The run is not a matched baseline comparison and cannot establish an end-to-end
improvement. Raw decoder totals include warmup; use first/last sampled deltas
for any interval comparison.

Warnings: the preceding UI native baseline recorded 0 Rust warnings and 1 Vite
large-chunk warning. Both focused Cargo logs and this native build contain
0 Rust warnings; this build retains 1 Vite large-chunk warning (delta 0).
`git diff --check` passed.
The same independent reviewer checked this note against the actual Cargo/build,
native measurement and close evidence and reported no blockers.

No physical monitor cadence, DMX/audio device, external receiver, or macOS
acceptance is inferred. The performance goal remains open; next comparisons must
align measured FPS and decoder diagnostic deltas within the same sample interval.
