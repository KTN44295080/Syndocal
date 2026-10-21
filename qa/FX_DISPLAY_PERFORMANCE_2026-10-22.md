# Display FX performance diagnosis

Base: shared `main`, `2907a2eac7fc487251153f6aa9d258c64564f5c4`.
This is a measurement checkpoint, **not a performance fix or release pass**.
No product implementation, output policy, schema or version changed.

The [previous runtime checkpoint](CHATGPT_RUNTIME_HANDOFF_2026-10-21.md)
recorded approximately 12.63 fps for an older native 3-layer/3-FX case, while
an isolated 1080p, one-chain ISF experiment took approximately 7 ms. The latter
is not a whole-output timing. The native fixture has three stages on **each**
of three layers (nine stages total), and actual output is 2560 x 1600.

## Current native measurements

Evidence root: `target/qa/fx-perf-2026-10-22/native/`.
Campaign: `31018a1c-21b3-4503-bed4-1130cc00db24`.

The unchanged source was built with the isolated campaign identifier and
`pnpm --dir app tauri build --no-bundle --config <campaign>/tauri.qa.conf.json`.
`build-01.log` records the exact MSVC 14.44.35207 pin and first-on-PATH linker;
`build-01.json` confirms unchanged HEAD/clean source around the build.
Copied QA executable SHA256:
`d41de3535a3c9465c0f9a13d3022411e349bc9d1895693759b5572385c48d5f2`.

Both fixtures use the same three enabled layers of the same silent 1920 x 1080,
30 fps H.264 file, and differ only in their layer ISF effects and output label.
Fixtures, media hashes, native process/CDP ancestry and main-window maximization
are recorded by the existing bounded performance harness. There were no
overlapping Cargo/rustc/link processes during measurement. Actual output geometry
was observed without a forced resize. Local native Display activation used its
actual confirmation dialogs; DMX routes remained disabled and Lighting permission
false. No audio input, camera, network video or external hardware was activated.

| Current source, actual 2560 x 1600 Display | 0 FX | 3 FX per layer |
| --- | ---: | ---: |
| Measurement interval | 19.486411 s | 59.491783 s |
| Native application submissions / interval | 1169 | 730 |
| Submission counter fps | 59.9905 | 12.2706 |
| Dropped-frame counter delta | 0 | 2840 |
| Prepare p50 / p95 | 0.6025 / 2.5894 ms | 78.3543 / 90.0274 ms |
| Native validation samples | 600 | 600 |
| Native frame-budget result | passed | **failed** |
| GPU composition layer slots | 3 | 0 |
| Decode wall mean per call over measured samples | 0.4795 ms | 20.9152 ms |
| Standby command wall time | 591.8931 ms | 583.4747 ms |

Reports: `<campaign>/perf-h264-1080-3l-01/evidence.json` and
`<campaign>/perf-h264-1080-3l-3fx-01/evidence.json`; derived `summary.json`.
Prepare excludes final GPU composition/present, so the two prepare columns do
not represent identical composition boundaries. Native counters establish local
application submission, not physical scanout or received FPS. The probe's
`passed: true` means observation and retirement completed; it does **not** turn
the FX frame-budget failure into a pass. The 0-FX 600-frame window had one
deadline miss, within its existing acceptance contract.

Both Stop operations returned Ready/Standby, denied both output domains, and
left no active routes/live windows. Both exact owned processes were closed via
their protected discard/stop dialog. Their fixture files were unchanged.
These single Stop observations are not latency ceilings against arbitrary
drivers or GPU/SDK stalls.

## Matched component measurements

A temporary `cfg(test)`-only harness ran two explicitly ignored release tests
with the same pinned compiler environment. Each command selected and passed
exactly one test; no wall-time assertion was added. After measurement, the
temporary module and its registration were removed from the product tree.
Its archived source is `native/temporary-cost-tests.rs`, SHA256
`789eb96414cb14c97ffe293070b2332844a363066e7b6e586566294f4b133ba4`.
The archived `.cmd` files require reinstating that temporary test registration
to reproduce; alone they no longer select tests in the clean product tree.

| Component, three layers, 12 warm samples | 1920 x 1080 p50 | 2560 x 1600 p50 |
| --- | ---: | ---: |
| Existing opaque Normal CPU blend, 8 row bands | 1.3922 ms | 2.8086 ms |
| Same CPU blend, 1 row band | 4.4785 ms | 9.6830 ms |
| Actual fixture's three-stage ISF per layer, all three layers, including input copies | 23.395 ms | 45.125 ms |

Logs: `native/cpu-cost-before.log`, `native/isf-matched-cost.log`.
The matched ISF p95 totals were 23.588 / 47.183 ms respectively. At actual output
size, individual apply medians were 12.925 / 12.531 / 12.834 ms; stack-runtime
medians were 12.348 / 11.797 / 12.264 ms. These include GPU upload, submission,
synchronous readback and host work; they are not isolated GPU kernel times and
exclude native decode/presentation. Do not sum percentiles or subtract these
separate experiments to invent an exact native time breakdown.

## Finding and next bounded work

The source and current counters agree: executing layer FX excludes native GPU
composition (`gpu_composition_eligible` / `try_prepare_gpu_composition` in
`crates/video/src/lib.rs`). The canonical preparation still batches stages
within each layer, but `isf_pass_runtime.rs` allocates upload/ping/pong/readback
resources and synchronously returns CPU pixels for each chain. The current
three-layer ISF boundary is substantially larger than the CPU blend alone.

The proposed opaque-row bulk-copy change was **not implemented**: its measured
upper scope was only about 2.8 ms of a roughly 78 ms prepare path. A simple GPU
eligibility relaxation was also not made: the existing path owns effect faults,
LastValid/blackout behavior and CPU/GPU frame boundaries, which need explicit
parity and failure proof before changing the contract.

Next: profile the matched chain's allocation/upload/wait/readback/copy phases,
then evaluate bounded GPU workspace reuse or a shared-device composition seam
with retained resource ownership, cancellation, budget, pixel, failure and
LastValid parity. Do not infer wasted work solely from zero lookahead hits:
`libav_decoder.rs` retains an earlier prefetched frame as the continuation's
terminal frame when the next request advances past it, along with the advanced
decoder session.

Web ChatGPT via Chat On Steroids performed one bounded read-only investigation
and stopped without edits, builds, app/device actions or extra workers. Its two
source candidates were independently checked by the supervisor and the
measurements above. Evidence: `native/web-investigation.txt` / `.png`,
conversation <https://chatgpt.com/c/6ad90ce4-df8c-83ec-9dad-973b3e7e3480>.

Rust warnings in the measured configurations: 0 (baseline 0, delta 0). The
existing Vite chunk notice remains 1 (baseline 1, delta 0). The original standard
executable was preserved and restored byte-for-byte, SHA256
`d493076c1fe137e523543cafaf3a7742d5ce52cf518aca0b264231c553adc371`.
Original profiles and previous evidence remain preserved. Hardware, other-OS,
network video, arbitrary-workload 60 fps and full release acceptance remain open.
